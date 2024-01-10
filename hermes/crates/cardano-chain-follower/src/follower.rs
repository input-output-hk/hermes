use std::sync::Arc;

use pallas::network::{facades::PeerClient, miniprotocols::Point};
use tokio::{
    sync::{mpsc, Mutex},
    task::JoinHandle,
};

use crate::{Error, MultiEraBlockData, Network, PointOrTip, Result};

/// Default [`Follower`] block buffer size.
const DEFAULT_CHAIN_UPDATE_BUFFER_SIZE: usize = 32;
/// Default [`Follower`] max await retries.
const DEFAULT_MAX_AWAIT_RETRIES: u32 = 3;

/// Enum of chain updates received by the follower.
pub enum ChainUpdate {
    /// New block inserted on chain.
    Block(MultiEraBlockData),
    /// Chain rollback to the given block.
    Rollback(MultiEraBlockData),
}

/// Builder used to create [`FollowerConfig`]s.
pub struct FollowerConfigBuilder {
    /// Block buffer size option.
    chain_update_buffer_size: usize,
    /// Maximum await retries option.
    max_await_retries: u32,
    /// Where to start following from.
    follow_from: PointOrTip,
}

impl Default for FollowerConfigBuilder {
    fn default() -> Self {
        Self {
            chain_update_buffer_size: DEFAULT_CHAIN_UPDATE_BUFFER_SIZE,
            max_await_retries: DEFAULT_MAX_AWAIT_RETRIES,
            follow_from: PointOrTip::Tip,
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

    /// Sets the maximum number of retries the [`Follower`] will execute when remote node
    /// sends an AWAIT message when the [`Follower`] is already in the "must reply"
    /// state.
    ///
    /// # Arguments
    ///
    /// * `max_await_retries`: Maximum number of retries.
    #[must_use]
    pub fn max_await_retries(mut self, max_await_retries: u32) -> Self {
        self.max_await_retries = max_await_retries;
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

    /// Builds a [`FollowerConfig`].
    #[must_use]
    pub fn build(self) -> FollowerConfig {
        FollowerConfig {
            chain_update_buffer_size: self.chain_update_buffer_size,
            max_await_retries: self.max_await_retries,
            follow_from: self.follow_from,
        }
    }
}

/// Configuration for the Cardano chain follower.
pub struct FollowerConfig {
    /// Configured chain update buffer size.
    pub chain_update_buffer_size: usize,
    /// Configured maximum await retry count.
    pub max_await_retries: u32,
    /// Where to start following from.
    pub follow_from: PointOrTip,
}

/// Cardano chain follower.
pub struct Follower {
    /// Client shared by the follower and its task.
    client: Arc<Mutex<PeerClient>>,
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
        let client = Arc::new(Mutex::new(
            PeerClient::connect(address, network.into())
                .await
                .map_err(Error::Client)?,
        ));

        let (chain_update_tx, chain_update_rx) = mpsc::channel(config.chain_update_buffer_size);

        let mut this = Self {
            client: client.clone(),
            chain_update_rx,
            task_join_handle: None,
        };

        let start_point = this
            .set_read_pointer(config.follow_from)
            .await?
            .ok_or(Error::FollowerStartPointNotFound)?;
        tracing::debug!(
            slot = start_point.slot_or_default(),
            "Follower read pointer set to starting point"
        );

        let task_join_handle = tokio::spawn(follow_task::run(
            client,
            chain_update_tx,
            config.max_await_retries,
        ));
        this.task_join_handle = Some(task_join_handle);

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
    pub async fn set_read_pointer<P>(&mut self, at: P) -> Result<Option<Point>>
    where
        P: Into<PointOrTip>,
    {
        let mut client = self.client.lock().await;

        match Into::<PointOrTip>::into(at) {
            PointOrTip::Point(Point::Origin) => {
                let point = client
                    .chainsync()
                    .intersect_origin()
                    .await
                    .map_err(Error::Chainsync)?;

                Ok(Some(point))
            },
            PointOrTip::Point(p @ Point::Specific(..)) => client
                .chainsync()
                .find_intersect(vec![p])
                .await
                .map(|(point, _)| point)
                .map_err(Error::Chainsync),
            PointOrTip::Tip => {
                let point = client
                    .chainsync()
                    .intersect_tip()
                    .await
                    .map_err(Error::Chainsync)?;

                Ok(Some(point))
            },
        }
    }

    /// Receive the next chain update from the producer.
    ///
    /// # Errors
    ///
    /// Returns Err if any producer communication errors occurred.
    #[allow(clippy::missing_panics_doc)]
    pub async fn next(&mut self) -> Result<ChainUpdate> {
        // This will not panic
        #[allow(clippy::expect_used)]
        self.chain_update_rx
            .recv()
            .await
            .expect("Follow task should be running")
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
    use tokio::sync::{mpsc, oneshot, Mutex};

    use crate::{Error, MultiEraBlockData};

    use super::ChainUpdate;

    /// Runs a [`Follower`](super::Follower) background task.
    ///
    /// The task runs until the chain update channel is closed (e.g. when the follower is
    /// dropped or the close fn is called).
    ///
    /// It keeps asking the connected node new chain updates. Every update and
    /// communication errors are sent through the channel to the follower.
    ///
    /// Backpressure is achieved with the channel's limited size.
    pub async fn run(
        client: Arc<Mutex<PeerClient>>, chain_update_tx: mpsc::Sender<crate::Result<ChainUpdate>>,
        max_retries_count: u32,
    ) {
        'main: loop {
            let try_count = 0;

            'tries: loop {
                assert!(try_count <= max_retries_count, "Node misbehavior");

                let (cancel_tx, _cancel_rx) = oneshot::channel::<()>();

                tokio::select! {
                    () = chain_update_tx.closed() => {
                        break 'main;
                    }

                    res = get_next_response(client.clone(), cancel_tx) => match res {
                        Err(err) => {
                            if chain_update_tx.send(Err(err)).await.is_err() {
                                break 'main;
                            }
                        },
                        Ok(next_response) => {
                            if let Some(chain_update) = next_response {
                                if chain_update_tx.send(Ok(chain_update)).await.is_err() {
                                    break 'tries;
                                }
                            }
                        }
                    }
                };
            }
        }

        tracing::debug!("Follower background task shutdown");
    }

    /// Waits for the next update from the node the client is connected to.
    ///
    /// Can be cancelled by closing the `cancel_tx` receiver end (explicitly or by
    /// dropping it).
    async fn get_next_response(
        client: Arc<Mutex<PeerClient>>, mut cancel_tx: oneshot::Sender<()>,
    ) -> crate::Result<Option<ChainUpdate>> {
        let res = {
            let mut client_lock = client.lock().await;

            if client_lock.chainsync().has_agency() {
                tokio::select! {
                    () = cancel_tx.closed() => { return Ok(None); }
                    res = client_lock.chainsync().request_next() => { res }
                }
            } else {
                tokio::select! {
                    () = cancel_tx.closed() => { return Ok(None); }
                    res = client_lock.chainsync().recv_while_must_reply() => { res }
                }
            }
            .map_err(Error::Chainsync)?
        };

        match res {
            chainsync::NextResponse::RollForward(header, _tip) => {
                let decoded_header = MultiEraHeader::decode(
                    header.variant,
                    header.byron_prefix.map(|p| p.0),
                    &header.cbor,
                )
                .map_err(Error::Codec)?;

                let mut client_lock = client.lock().await;

                let req_fut = client_lock.blockfetch().fetch_single(Point::Specific(
                    decoded_header.slot(),
                    decoded_header.hash().to_vec(),
                ));

                let block_data = tokio::select! {
                    () = cancel_tx.closed() => { return Ok(None); }
                    res = req_fut => { res.map_err(Error::Blockfetch)? }
                };

                Ok(Some(ChainUpdate::Block(MultiEraBlockData(block_data))))
            },
            chainsync::NextResponse::RollBackward(point, _tip) => {
                let mut client_lock = client.lock().await;

                let req_fut = client_lock.blockfetch().fetch_single(point);

                let block_data = tokio::select! {
                    () = cancel_tx.closed() => { return Ok(None); }
                    res = req_fut => { res.map_err(Error::Blockfetch)? }
                };

                Ok(Some(ChainUpdate::Rollback(MultiEraBlockData(block_data))))
            },
            chainsync::NextResponse::Await => Ok(None),
        }
    }
}
