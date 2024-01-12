use std::path::PathBuf;

use pallas::network::{facades::PeerClient, miniprotocols::Point};
use tokio::{
    sync::{mpsc, oneshot},
    task::JoinHandle,
};

use crate::{
    mithril_snapshot::MithrilSnapshot, Error, MultiEraBlockData, Network, PointOrTip, Result,
};

/// Default [`Follower`] block buffer size.
const DEFAULT_CHAIN_UPDATE_BUFFER_SIZE: usize = 32;

/// Enum of chain updates received by the follower.
pub enum ChainUpdate {
    /// New block inserted on chain.
    Block(MultiEraBlockData),
    /// Chain rollback to the given block.
    Rollback(MultiEraBlockData),
}

impl ChainUpdate {
    /// ...
    pub fn block_data(&self) -> &MultiEraBlockData {
        match self {
            ChainUpdate::Block(block_data) => block_data,
            ChainUpdate::Rollback(block_data) => block_data,
        }
    }
}

/// Builder used to create [`FollowerConfig`]s.
pub struct FollowerConfigBuilder {
    /// Block buffer size option.
    chain_update_buffer_size: usize,
    /// Where to start following from.
    follow_from: PointOrTip,
    /// ...
    mithril_snapshot_path: Option<PathBuf>,
}

impl Default for FollowerConfigBuilder {
    fn default() -> Self {
        Self {
            chain_update_buffer_size: DEFAULT_CHAIN_UPDATE_BUFFER_SIZE,
            follow_from: PointOrTip::Tip,
            mithril_snapshot_path: None,
        }
    }
}

impl FollowerConfigBuilder {
    /// Sets the size of the chain updates buffer used by the [`Follower`].
    ///
    /// # Arguments
    ///
    /// * `chain_update_buffer_size`: Size of the chain updates buffer.
    #[must_use]
    pub fn chain_update_buffer_size(mut self, block_buffer_size: usize) -> Self {
        self.chain_update_buffer_size = block_buffer_size;
        self
    }

    /// Sets the point at which the follower will start following from.
    ///
    /// # Arguments
    ///
    /// * `from`: Sync starting point.
    #[must_use]
    pub fn follow_from<P>(mut self, from: P) -> Self
    where
        P: Into<PointOrTip>,
    {
        self.follow_from = from.into();
        self
    }

    /// Sets the path of the Mithril snapshot the [`Follower`] will use.
    ///
    /// # Arguments
    ///
    /// * `path`: Mithril snapshot path.
    #[must_use]
    pub fn mithril_snapshot_path(mut self, path: PathBuf) -> Self {
        self.mithril_snapshot_path = Some(path);
        self
    }

    /// Builds a [`FollowerConfig`].
    #[must_use]
    pub fn build(self) -> FollowerConfig {
        FollowerConfig {
            chain_update_buffer_size: self.chain_update_buffer_size,
            follow_from: self.follow_from,
            mithril_snapshot_path: self.mithril_snapshot_path,
        }
    }
}

/// Configuration for the Cardano chain follower.
pub struct FollowerConfig {
    /// Configured chain update buffer size.
    pub chain_update_buffer_size: usize,
    /// Where to start following from.
    pub follow_from: PointOrTip,
    /// ...
    pub mithril_snapshot_path: Option<PathBuf>,
}

/// Cardano chain follower.
pub struct Follower {
    /// Task request sender.
    task_request_tx: mpsc::Sender<(follow_task::Request, oneshot::Sender<follow_task::Response>)>,
    /// Chain update receiver.
    chain_update_rx: mpsc::Receiver<Result<ChainUpdate>>,
    /// Task thread join handle.
    task_join_handle: Option<JoinHandle<()>>,
}

impl Follower {
    /// Connects the follower to a producer using the node-to-node protocol.
    ///
    /// # Arguments
    ///
    /// * `address`: Address of the node to connect to.
    /// * `network`: The [Network] the client is assuming it's connecting to.
    /// * `config`: Follower's configuration (see [`FollowerConfigBuilder`]).
    ///
    /// # Errors
    ///
    /// Returns Err if the connection could not be established.
    pub async fn connect(address: &str, network: Network, config: FollowerConfig) -> Result<Self> {
        let client = PeerClient::connect(address, network.into())
            .await
            .map_err(Error::Client)?;

        let (task_request_tx, task_request_rx) = mpsc::channel(16);
        let (chain_update_tx, chain_update_rx) = mpsc::channel(config.chain_update_buffer_size);

        let mithril_snapshot = config
            .mithril_snapshot_path
            .and_then(MithrilSnapshot::from_path);

        let task_join_handle = tokio::spawn(follow_task::run(
            client,
            mithril_snapshot,
            task_request_rx,
            chain_update_tx,
        ));

        let this = Self {
            task_request_tx,
            chain_update_rx,
            task_join_handle: Some(task_join_handle),
        };

        this.set_read_pointer(config.follow_from)
            .await?
            .ok_or(Error::FollowerStartPointNotFound)?;

        Ok(this)
    }

    /// Set the follower's chain read-pointer. Returns None if the point was
    /// not found on the chain.
    ///
    /// # Arguments
    ///
    /// * `at`: Point at which to set the read-pointer.
    ///
    /// # Errors
    ///
    /// Returns Err if something went wrong while communicating with the producer.
    pub async fn set_read_pointer<P>(&self, at: P) -> Result<Option<Point>>
    where
        P: Into<PointOrTip>,
    {
        let res = self.send_request_and_wait(follow_task::Request::SetReadPointer(at.into()));

        let follow_task::Response::SetReadPointer(res) = res.await?;
        res
    }

    /// Receive the next chain update from the producer.
    ///
    /// # Errors
    ///
    /// Returns Err if any producer communication errors occurred.
    #[allow(clippy::missing_panics_doc)]
    pub async fn next(&mut self) -> Result<ChainUpdate> {
        self.chain_update_rx
            .recv()
            .await
            .ok_or(Error::FollowerBackgroundTaskNotRunning)?
    }

    /// Closes the follower connection and stops its background task.
    ///
    /// # Errors
    ///
    /// Returns Err if some error occurred in the background task.
    pub async fn close(mut self) -> std::result::Result<(), tokio::task::JoinError> {
        self.chain_update_rx.close();

        if let Some(join_handle) = self.task_join_handle {
            join_handle.await
        } else {
            Ok(())
        }
    }

    async fn send_request_and_wait(
        &self, req: follow_task::Request,
    ) -> Result<follow_task::Response> {
        let (response_tx, response_rx) = oneshot::channel();

        self.task_request_tx
            .send((req, response_tx))
            .await
            .map_err(|_| Error::FollowerBackgroundTaskNotRunning)?;

        response_rx
            .await
            .map_err(|_| Error::FollowerBackgroundTaskNotRunning)
    }
}

/// Contains functions related to the Follower's background task.
mod follow_task {
    use std::sync::Arc;

    use pallas::{
        ledger::traverse::MultiEraHeader,
        network::{
            facades::PeerClient,
            miniprotocols::{chainsync, Point},
        },
    };
    use tokio::sync::{mpsc, oneshot, Mutex, Notify, RwLock};

    use crate::{
        mithril_snapshot::{MithrilSnapshot, MithrilSnapshotIterator},
        Error, MultiEraBlockData, PointOrTip, Result,
    };

    use super::ChainUpdate;

    pub enum Request {
        SetReadPointer(PointOrTip),
    }

    pub enum Response {
        SetReadPointer(Result<Option<Point>>),
    }

    struct MithrilSnapshotState {
        snapshot: MithrilSnapshot,
        iter: Option<MithrilSnapshotIterator>,
    }

    #[derive(Clone)]
    pub(crate) struct TaskState {
        client: Arc<Mutex<PeerClient>>,
        chain_update_tx: mpsc::Sender<crate::Result<ChainUpdate>>,
        current_read_pointer: Arc<RwLock<Option<Point>>>,
        current_read_pointer_notify: Arc<Notify>,
        mithril_snapshot_state: Arc<RwLock<Option<MithrilSnapshotState>>>,
    }

    /// Runs a [`Follower`](super::Follower) background task.
    ///
    /// The task runs until the chain update channel is closed (e.g. when the follower is
    /// dropped or the close fn is called).
    ///
    /// It keeps asking the connected node new chain updates. Every update and
    /// communication errors are sent through the channel to the follower.
    ///
    /// Backpressure is achieved with the channel's limited size.
    pub(crate) async fn run(
        client: PeerClient, mithril_snapshot: Option<MithrilSnapshot>,
        mut request_rx: mpsc::Receiver<(Request, oneshot::Sender<Response>)>,
        chain_update_tx: mpsc::Sender<crate::Result<ChainUpdate>>,
    ) {
        let mithril_snapshot_state = mithril_snapshot.map(|snapshot| MithrilSnapshotState {
            snapshot,
            iter: None,
        });

        let task_state = TaskState {
            client: Arc::new(Mutex::new(client)),
            chain_update_tx,
            current_read_pointer: Arc::new(RwLock::new(None)),
            current_read_pointer_notify: Arc::new(Notify::new()),
            mithril_snapshot_state: Arc::new(RwLock::new(mithril_snapshot_state)),
        };

        'main: loop {
            tokio::select! {
                () = task_state.chain_update_tx.closed() => break 'main,

                Some((req, res_channel)) = request_rx.recv() => {
                    handle_request(task_state.clone(), req, res_channel).await;
                }

                res = send_next(task_state.clone()) => {
                    if res.is_err() {
                        break 'main;
                    }
                }
            }
        }

        tracing::trace!("Follower background task shutdown");
    }

    async fn handle_request(
        state: TaskState, request: Request, response_channel: oneshot::Sender<Response>,
    ) {
        match request {
            Request::SetReadPointer(at) => {
                tracing::trace!("Setting follower's read point");

                let mut current_read_pointer_lock = state.current_read_pointer.write().await;
                let mut client = state.client.lock().await;
                let mut mithril_snapshot_lock = state.mithril_snapshot_state.write().await;

                let set_to_snapshot = if let Some(s) = mithril_snapshot_lock.as_mut() {
                    if let PointOrTip::Point(p) = &at {
                        s.iter = s.snapshot.try_read_blocks_from_point(p.clone());
                        s.iter.as_ref().map(|_| p.clone())
                    } else {
                        s.iter = None;
                        None
                    }
                } else {
                    None
                };

                let result = if set_to_snapshot.is_some() {
                    tracing::trace!("Found point in Mithril snapshot");
                    Ok(set_to_snapshot)
                } else {
                    if mithril_snapshot_lock.is_some() {
                        tracing::trace!("Point not found in Mithril snapshot. Asking remote node.");
                    }

                    set_client_read_pointer(&mut client, at).await
                };

                match result.as_ref() {
                    Ok(Some(point)) => {
                        tracing::trace!(slot = point.slot_or_default(), "Read pointer set");
                        *current_read_pointer_lock = Some(point.clone());
                    },
                    _ => *current_read_pointer_lock = None,
                }

                drop(response_channel.send(Response::SetReadPointer(result)));
            },
        }
    }

    async fn send_next(
        state: TaskState,
    ) -> std::result::Result<(), mpsc::error::SendError<Result<ChainUpdate>>> {
        // Get the value of the current read pointer
        let mut current_read_pointer_lock = state.current_read_pointer.write().await;

        let mut next = Ok(None);

        {
            // If we have no valid read pointer:
            // 1. drop the lock
            // 2. wait for a notification indicating it has been set
            // 3. check the lock again.
            let current_point = loop {
                if let Some(current_point) = current_read_pointer_lock.as_ref() {
                    break current_point;
                }

                drop(current_read_pointer_lock);

                tracing::trace!("Waiting for a valid read pointer to be set");
                state.current_read_pointer_notify.notified().await;

                current_read_pointer_lock = state.current_read_pointer.write().await;
            };

            let mut mithril_snapshot_lock = state.mithril_snapshot_state.write().await;

            if let Some(s) = mithril_snapshot_lock.as_mut() {
                if let Some(iter) = s.iter.as_mut() {
                    if let Some(b) = iter.next() {
                        tracing::trace!("Read block data from Mithril snapshot");

                        next = b
                            .map(|b| Some(ChainUpdate::Block(MultiEraBlockData(b))))
                            .map_err(|_| Error::MithrilSnapshot);
                    }
                }
            }

            if let Ok(None) = next {
                if let Some(s) = mithril_snapshot_lock.as_mut() {
                    if s.iter.is_some() {
                        s.iter = None;

                        {
                            let mut client = state.client.lock().await;
                            let res = set_client_read_pointer(
                                &mut client,
                                PointOrTip::Point(current_point.clone()),
                            )
                            .await;

                            if let Err(e) = res {
                                *current_read_pointer_lock = None;
                                return state.chain_update_tx.send(Err(e)).await;
                            }
                        }

                        // Skip the next update from the client since we've already
                        // read it the Mithril snapshot.
                        drop(next_from_client(state.clone()).await);
                    }
                }

                next = next_from_client(state.clone()).await;
            }
        };

        match next {
            Err(err) => {
                return state.chain_update_tx.send(Err(err)).await;
            },
            Ok(next_response) => {
                if let Some(chain_update) = next_response {
                    let block_data = chain_update.block_data();

                    let block = match block_data.decode() {
                        Ok(decoded_block) => decoded_block,
                        Err(e) => return state.chain_update_tx.send(Err(e)).await,
                    };

                    tracing::trace!(slot = block.slot(), "Read pointer updated");
                    *current_read_pointer_lock =
                        Some(Point::Specific(block.slot(), block.hash().to_vec()));

                    return state.chain_update_tx.send(Ok(chain_update)).await;
                }
            },
        }

        Ok(())
    }

    /// Waits for the next update from the node the client is connected to.
    ///
    /// Is cancelled by closing the `chain_update_tx` receiver end (explicitly or by
    /// dropping it).
    async fn next_from_client(state: TaskState) -> crate::Result<Option<ChainUpdate>> {
        let res = {
            let mut client_lock = state.client.lock().await;

            if client_lock.chainsync().has_agency() {
                tokio::select! {
                    () = state.chain_update_tx.closed() => { return Ok(None); }
                    res = client_lock.chainsync().request_next() => { res }
                }
            } else {
                tokio::select! {
                    () = state.chain_update_tx.closed() => { return Ok(None); }
                    res = client_lock.chainsync().recv_while_must_reply() => { res }
                }
            }
            .map_err(Error::Chainsync)?
        };

        tracing::trace!("Received block data from client");

        match res {
            chainsync::NextResponse::RollForward(header, _tip) => {
                let decoded_header = MultiEraHeader::decode(
                    header.variant,
                    header.byron_prefix.map(|p| p.0),
                    &header.cbor,
                )
                .map_err(Error::Codec)?;

                let mut client_lock = state.client.lock().await;

                let req_fut = client_lock.blockfetch().fetch_single(Point::Specific(
                    decoded_header.slot(),
                    decoded_header.hash().to_vec(),
                ));

                let block_data = tokio::select! {
                    () = state.chain_update_tx.closed() => { return Ok(None); }
                    res = req_fut => { res.map_err(Error::Blockfetch)? }
                };

                Ok(Some(ChainUpdate::Block(MultiEraBlockData(block_data))))
            },
            chainsync::NextResponse::RollBackward(point, _tip) => {
                let mut client_lock = state.client.lock().await;

                let req_fut = client_lock.blockfetch().fetch_single(point);

                let block_data = tokio::select! {
                    () = state.chain_update_tx.closed() => { return Ok(None); }
                    res = req_fut => { res.map_err(Error::Blockfetch)? }
                };

                Ok(Some(ChainUpdate::Rollback(MultiEraBlockData(block_data))))
            },
            chainsync::NextResponse::Await => Ok(None),
        }
    }

    async fn set_client_read_pointer(
        client: &mut PeerClient, at: PointOrTip,
    ) -> Result<Option<Point>> {
        match at {
            PointOrTip::Point(Point::Origin) => client
                .chainsync()
                .intersect_origin()
                .await
                .map(Some)
                .map_err(Error::Chainsync),
            PointOrTip::Point(p @ Point::Specific(..)) => client
                .chainsync()
                .find_intersect(vec![p])
                .await
                .map(|(point, _)| point)
                .map_err(Error::Chainsync),
            PointOrTip::Tip => client
                .chainsync()
                .intersect_tip()
                .await
                .map(Some)
                .map_err(Error::Chainsync),
        }
    }
}
