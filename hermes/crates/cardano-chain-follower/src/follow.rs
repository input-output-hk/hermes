//! Cardano chain follow module.

use std::{future::Future, path::PathBuf};

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
    /// Gets the chain update's block data.
    #[must_use]
    pub fn block_data(&self) -> &MultiEraBlockData {
        match self {
            ChainUpdate::Block(block_data) | ChainUpdate::Rollback(block_data) => block_data,
        }
    }
}

/// Builder used to create [`FollowerConfig`]s.
pub struct FollowerConfigBuilder {
    /// Block buffer size option.
    chain_update_buffer_size: usize,
    /// Where to start following from.
    follow_from: PointOrTip,
    /// Path to the Mithril snapshot the follower should use.
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
    where P: Into<PointOrTip> {
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
#[derive(Clone)]
pub struct FollowerConfig {
    /// Configured chain update buffer size.
    pub chain_update_buffer_size: usize,
    /// Where to start following from.
    pub follow_from: PointOrTip,
    /// Path to the Mithril snapshot the follower should use.
    pub mithril_snapshot_path: Option<PathBuf>,
}

/// Information used to connect to a client.
#[derive(Clone)]
struct ClientConnectInfo {
    /// Node's address
    address: String,
    /// Network magic
    network: Network,
}

/// Handler for receiving the read block response from the client.
pub struct ReadBlock(tokio::task::JoinHandle<Result<MultiEraBlockData>>);

impl Future for ReadBlock {
    type Output = Result<MultiEraBlockData>;

    fn poll(
        mut self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let p = &mut self.0;
        // Using tokio pin instead of, e.g., pin-project because we use tokio as the async runtime
        // lib for this crate.
        tokio::pin!(p);

        match p.poll(cx) {
            std::task::Poll::Ready(res) => {
                match res {
                    Ok(res) => std::task::Poll::Ready(res),
                    Err(_) => std::task::Poll::Ready(Err(Error::InternalError)),
                }
            },
            std::task::Poll::Pending => std::task::Poll::Pending,
        }
    }
}

/// Handler for receiving the read block range response from the client.
pub struct ReadBlockRange(tokio::task::JoinHandle<Result<Vec<MultiEraBlockData>>>);

impl Future for ReadBlockRange {
    type Output = Result<Vec<MultiEraBlockData>>;

    fn poll(
        mut self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let p = &mut self.0;
        // Using tokio pin instead of, e.g., pin-project because we use tokio as the async runtime
        // lib for this crate.
        tokio::pin!(p);

        match p.poll(cx) {
            std::task::Poll::Ready(res) => {
                match res {
                    Ok(res) => std::task::Poll::Ready(res),
                    Err(_) => std::task::Poll::Ready(Err(Error::InternalError)),
                }
            },
            std::task::Poll::Pending => std::task::Poll::Pending,
        }
    }
}

/// Cardano chain follower.
pub struct Follower {
    /// Client connection information.
    ///
    /// This is used to open more connections when needed.
    client_connect_info: ClientConnectInfo,
    /// Chain update receiver.
    chain_update_rx: mpsc::Receiver<Result<ChainUpdate>>,
    /// Follow task request sender.
    follow_task_request_tx: mpsc::Sender<task::SetReadPointerRequest>,
    /// Follow task thread join handle.
    follow_task_join_handle: JoinHandle<()>,
    /// Optional Mithril snapshot information.
    mithril_snapshot: Option<MithrilSnapshot>,
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
        let mut client = PeerClient::connect(address, network.into())
            .await
            .map_err(Error::Client)?;

        let Some(follow_from) = set_client_read_pointer(&mut client, config.follow_from).await?
        else {
            return Err(Error::SetReadPointer);
        };

        let mithril_snapshot = if let Some(path) = config.mithril_snapshot_path {
            Some(MithrilSnapshot::from_path(path)?)
        } else {
            None
        };

        let connect_info = ClientConnectInfo {
            address: address.to_string(),
            network,
        };

        let (task_request_tx, chain_update_rx, task_join_handle) = task::FollowTask::spawn(
            client,
            connect_info,
            mithril_snapshot.clone(),
            config.chain_update_buffer_size,
            follow_from,
        );

        let client_connect_info = ClientConnectInfo {
            address: address.to_string(),
            network,
        };

        Ok(Self {
            client_connect_info,
            chain_update_rx,
            follow_task_request_tx: task_request_tx,
            follow_task_join_handle: task_join_handle,
            mithril_snapshot,
        })
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
    where P: Into<PointOrTip> {
        let (response_tx, response_rx) = oneshot::channel();

        let req = task::SetReadPointerRequest {
            at: at.into(),
            response_tx,
        };

        self.follow_task_request_tx
            .send(req)
            .await
            .map_err(|_| Error::FollowTaskNotRunning)?;

        response_rx.await.map_err(|_| Error::FollowTaskNotRunning)?
    }

    /// Requests the client to read a block.
    ///
    /// # Arguments
    ///
    /// * `at`: Point at which to read the block.
    #[must_use]
    pub fn read_block<P>(&self, at: P) -> ReadBlock
    where P: Into<PointOrTip> {
        let connect_info = self.client_connect_info.clone();
        let mithril_snapshot = self.mithril_snapshot.clone();
        let at = at.into();

        let join_handle = tokio::spawn(async move {
            let mut client = PeerClient::connect(connect_info.address, connect_info.network.into())
                .await
                .map_err(Error::Client)?;

            match at {
                PointOrTip::Tip => {
                    let point = resolve_tip(&mut client).await?;
                    read_block_from_network(&mut client, point).await
                },

                PointOrTip::Point(point) => {
                    let snapshot_res = mithril_snapshot
                        .as_ref()
                        .and_then(|snapshot| snapshot.try_read_block(point.clone()).ok())
                        .flatten();

                    match snapshot_res {
                        Some(block_data) => {
                            tracing::trace!("Read block from Mithril snapshot");
                            Ok(block_data)
                        },
                        None => read_block_from_network(&mut client, point).await,
                    }
                },
            }
        });

        ReadBlock(join_handle)
    }

    /// Request the client to read a block range.
    ///
    /// # Arguments
    ///
    /// * `from`: Block range start.
    /// * `to`: Block range end.
    #[must_use]
    pub fn read_block_range<P>(&self, from: Point, to: P) -> ReadBlockRange
    where P: Into<PointOrTip> {
        let connect_info = self.client_connect_info.clone();
        let mithril_snapshot = self.mithril_snapshot.clone();
        let to = to.into();

        let join_handle = tokio::spawn(async move {
            let mut client = PeerClient::connect(connect_info.address, connect_info.network.into())
                .await
                .map_err(Error::Client)?;

            match to {
                PointOrTip::Tip => {
                    let to_point = resolve_tip(&mut client).await?;
                    read_block_range_from_network(&mut client, from, to_point).await
                },
                PointOrTip::Point(to) => {
                    let snapshot_res = mithril_snapshot
                        .as_ref()
                        .and_then(|snapshot| {
                            snapshot.try_read_block_range(from.clone(), to.clone()).ok()
                        })
                        .flatten();

                    match snapshot_res {
                        Some((last_point_read, mut block_data_vec)) => {
                            // If we couldn't get all the blocks from the snapshot,
                            // try fetching the remaining ones from the network.
                            if last_point_read.slot_or_default() < to.slot_or_default() {
                                let network_blocks =
                                    read_block_range_from_network(&mut client, last_point_read, to)
                                        .await?;

                                // Discard 1st point as it's already been read from
                                // the snapshot
                                let mut network_blocks_iter = network_blocks.into_iter();
                                drop(network_blocks_iter.next());

                                block_data_vec.extend(network_blocks_iter);
                            }

                            Ok(block_data_vec)
                        },
                        None => read_block_range_from_network(&mut client, from, to).await,
                    }
                },
            }
        });

        ReadBlockRange(join_handle)
    }

    /// Receive the next chain update from the producer.
    ///
    /// # Errors
    ///
    /// Returns Err if any producer communication errors occurred.
    pub async fn next(&mut self) -> Result<ChainUpdate> {
        self.chain_update_rx
            .recv()
            .await
            .ok_or(Error::FollowTaskNotRunning)?
    }

    /// Closes the follower connection and stops its background task.
    ///
    /// # Errors
    ///
    /// Returns Err if some error occurred in the background task.
    pub async fn close(self) -> std::result::Result<(), tokio::task::JoinError> {
        // NOTE(FelipeRosa): For now just abort the task since it needs no cancellation
        self.follow_task_join_handle.abort();

        self.follow_task_join_handle.await
    }
}

/// Contains functions related to the Follower's background task.
mod task {
    use pallas::{
        ledger::traverse::MultiEraHeader,
        network::{
            facades::PeerClient,
            miniprotocols::{chainsync, Point},
        },
    };
    use tokio::sync::{mpsc, oneshot};

    use super::{set_client_read_pointer, ChainUpdate, ClientConnectInfo};
    use crate::{mithril_snapshot::MithrilSnapshot, Error, MultiEraBlockData, PointOrTip, Result};

    /// Request the task to set the read pointer to the given point or to the
    /// tip.
    pub(super) struct SetReadPointerRequest {
        /// Point at which to set the read pointer.
        pub(super) at: PointOrTip,
        /// The channel that will be used to send the request's response.
        pub(super) response_tx: oneshot::Sender<Result<Option<Point>>>,
    }

    /// Holds state for a follow task.
    pub(super) struct FollowTask {
        /// Client connection info.
        connect_info: ClientConnectInfo,
        /// Optional Mithril Snapshot that will be used by the follow task when fetching
        /// chain updates.
        mithril_snapshot: Option<MithrilSnapshot>,
        /// Request receiver.
        request_rx: mpsc::Receiver<SetReadPointerRequest>,
        /// Chain update sender.
        chain_update_tx: mpsc::Sender<crate::Result<ChainUpdate>>,
    }

    impl FollowTask {
        /// Spawn a follow task.
        pub(super) fn spawn(
            client: PeerClient, connect_info: ClientConnectInfo,
            mithril_snapshot: Option<MithrilSnapshot>, buffer_size: usize, follow_from: Point,
        ) -> (
            mpsc::Sender<SetReadPointerRequest>,
            mpsc::Receiver<crate::Result<ChainUpdate>>,
            tokio::task::JoinHandle<()>,
        ) {
            let (request_tx, request_rx) = mpsc::channel(1);
            let (chain_update_tx, chain_update_rx) = mpsc::channel(buffer_size);

            let this = Self {
                connect_info,
                mithril_snapshot,
                request_rx,
                chain_update_tx,
            };

            (
                request_tx,
                chain_update_rx,
                tokio::spawn(this.run(client, follow_from)),
            )
        }

        /// Runs the follow task.
        ///
        /// It keeps asking the connected node for new chain updates. Every update and
        /// communication errors are sent through the channel to the follower.
        ///
        /// Backpressure is achieved with the chain update channel's limited size.
        async fn run(mut self, client: PeerClient, from: Point) {
            let fetch_chain_updates_fut = Self::fetch_chain_updates(
                client,
                self.mithril_snapshot.as_ref(),
                self.chain_update_tx.clone(),
                from,
            );
            tokio::pin!(fetch_chain_updates_fut);

            loop {
                tokio::select! {
                    Some(SetReadPointerRequest { at, response_tx }) = self.request_rx.recv() => {
                        let res = PeerClient::connect(&self.connect_info.address, self.connect_info.network.into())
                            .await;

                        let Ok(mut client) = res else {
                            drop(response_tx.send(Err(crate::Error::SetReadPointer)));
                            continue;
                        };

                        match set_client_read_pointer(&mut client, at).await {
                            Ok(Some(from)) => {
                                fetch_chain_updates_fut.set(Self::fetch_chain_updates(
                                    client,
                                    self.mithril_snapshot.as_ref(),
                                    self.chain_update_tx.clone(),
                                    from.clone(),
                                ));

                                drop(response_tx.send(Ok(Some(from))));
                            }
                            Ok(None) => {
                                drop(response_tx.send(Ok(None)));
                            }
                            Err(_) => {
                                drop(response_tx.send(Err(crate::Error::SetReadPointer)));
                                continue;
                            }
                        }
                    }

                    () = &mut fetch_chain_updates_fut  => {}
                }
            }
        }

        /// Sends the next chain update to the follower.
        /// This can be either read from the Mithril snapshot (if configured) or
        /// from the N2N remote client.
        async fn fetch_chain_updates(
            mut client: PeerClient, mithril_snapshot: Option<&MithrilSnapshot>,
            chain_update_tx: mpsc::Sender<crate::Result<ChainUpdate>>, from: Point,
        ) {
            let mut current_point = from;

            let set_to_snapshot = mithril_snapshot
                .and_then(|snapshot| snapshot.try_read_blocks_from_point(current_point.clone()));

            if let Some(iter) = set_to_snapshot {
                let mut last_recv_from_snapshot = false;

                for result in iter {
                    let mut fallback = false;

                    if let Ok(raw_block_data) = result {
                        let block_data = MultiEraBlockData(raw_block_data);

                        match block_data.decode() {
                            Ok(block) => {
                                current_point =
                                    Point::Specific(block.slot(), block.hash().to_vec());

                                if chain_update_tx
                                    .send(Ok(ChainUpdate::Block(block_data)))
                                    .await
                                    .is_err()
                                {
                                    return;
                                }

                                last_recv_from_snapshot = true;
                            },
                            Err(_) => {
                                fallback = true;
                            },
                        }
                    } else {
                        fallback = true;
                    }

                    // If we, for any reason, we failed to get the block from the
                    // Mithril snapshot, fallback to the getting it from the client.
                    if fallback {
                        let res = set_client_read_pointer(
                            &mut client,
                            PointOrTip::Point(current_point.clone()),
                        )
                        .await;

                        match res {
                            Ok(Some(p)) => {
                                current_point = p;

                                if !Self::send_next_chain_update(
                                    &mut client,
                                    chain_update_tx.clone(),
                                )
                                .await
                                {
                                    return;
                                }
                            },
                            Ok(None) | Err(_) => {
                                drop(
                                    chain_update_tx
                                        .send(Err(crate::Error::SetReadPointer))
                                        .await,
                                );
                                return;
                            },
                        }
                    }
                }

                if last_recv_from_snapshot {
                    let res = set_client_read_pointer(
                        &mut client,
                        PointOrTip::Point(current_point.clone()),
                    )
                    .await;

                    if let Err(e) = res {
                        drop(chain_update_tx.send(Err(e)).await);
                        return;
                    }

                    // Skip the next update from the client since we've already
                    // read it the Mithril snapshot.
                    drop(Self::next_from_client(&mut client).await);
                }
            }

            while Self::send_next_chain_update(&mut client, chain_update_tx.clone()).await {}
        }

        /// Waits for the next update from the node the client is connected to.
        ///
        /// Is cancelled by closing the `chain_update_tx` receiver end (explicitly or by
        /// dropping it).
        async fn next_from_client(client: &mut PeerClient) -> crate::Result<Option<ChainUpdate>> {
            tracing::trace!("Requesting next chain update");
            let res = {
                match client.chainsync().state() {
                    chainsync::State::CanAwait => client.chainsync().recv_while_can_await().await,
                    chainsync::State::MustReply => client.chainsync().recv_while_must_reply().await,
                    _ => client.chainsync().request_next().await,
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

                    let point =
                        Point::Specific(decoded_header.slot(), decoded_header.hash().to_vec());
                    tracing::trace!(point = ?point, "Fetching roll forward block data");
                    let block_data = client
                        .blockfetch()
                        .fetch_single(point)
                        .await
                        .map_err(Error::Blockfetch)?;

                    Ok(Some(ChainUpdate::Block(MultiEraBlockData(block_data))))
                },
                chainsync::NextResponse::RollBackward(point, _tip) => {
                    tracing::trace!(point = ?point, "Fetching roll backward block data");
                    let block_data = client
                        .blockfetch()
                        .fetch_single(point)
                        .await
                        .map_err(Error::Blockfetch)?;

                    Ok(Some(ChainUpdate::Rollback(MultiEraBlockData(block_data))))
                },
                chainsync::NextResponse::Await => Ok(None),
            }
        }

        /// Sends the next chain update through the follower's chain update channel.
        async fn send_next_chain_update(
            client: &mut PeerClient, chain_update_tx: mpsc::Sender<crate::Result<ChainUpdate>>,
        ) -> bool {
            loop {
                let res = Self::next_from_client(client).await;

                match res {
                    Err(err) => {
                        if chain_update_tx.send(Err(err)).await.is_err() {
                            return false;
                        }
                    },
                    Ok(next_response) => {
                        if let Some(chain_update) = next_response {
                            if chain_update_tx.send(Ok(chain_update)).await.is_err() {
                                return false;
                            }

                            return true;
                        }
                    },
                }
            }
        }
    }
}

/// Sets the N2N remote client's read pointer.
async fn set_client_read_pointer(client: &mut PeerClient, at: PointOrTip) -> Result<Option<Point>> {
    match at {
        PointOrTip::Point(Point::Origin) => {
            client
                .chainsync()
                .intersect_origin()
                .await
                .map(Some)
                .map_err(Error::Chainsync)
        },
        PointOrTip::Point(p @ Point::Specific(..)) => {
            client
                .chainsync()
                .find_intersect(vec![p])
                .await
                .map(|(point, _)| point)
                .map_err(Error::Chainsync)
        },
        PointOrTip::Tip => {
            client
                .chainsync()
                .intersect_tip()
                .await
                .map(Some)
                .map_err(Error::Chainsync)
        },
    }
}

/// Finds the tip point.
///
/// NOTE: This changes the client's read pointer position.
#[inline]
async fn resolve_tip(client: &mut PeerClient) -> Result<Point> {
    client
        .chainsync()
        .intersect_tip()
        .await
        .map_err(Error::Chainsync)
}

/// Reads a block from the network using the N2N client.
async fn read_block_from_network(
    blockfetch_client: &mut PeerClient, point: Point,
) -> Result<MultiEraBlockData> {
    // Used in tracing
    let slot = point.slot_or_default();

    let block_data = blockfetch_client
        .blockfetch()
        .fetch_single(point)
        .await
        .map_err(Error::Blockfetch)?;

    tracing::trace!(slot, "Block read from n2n");
    Ok(MultiEraBlockData(block_data))
}

/// Reads a range of blocks from the network using the N2N client.
async fn read_block_range_from_network(
    blockfetch_client: &mut PeerClient, from: Point, to: Point,
) -> Result<Vec<MultiEraBlockData>> {
    // Used in tracing
    let from_slot = from.slot_or_default();
    let to_slot = to.slot_or_default();

    let data_vec = blockfetch_client
        .blockfetch()
        .fetch_range((from, to))
        .await
        .map_err(Error::Blockfetch)?
        .into_iter()
        .map(MultiEraBlockData)
        .collect();

    tracing::trace!(from_slot, to_slot, "Block range read from n2n");

    Ok(data_vec)
}
