//! Cardano chain follow module.

use std::{future::Future, path::PathBuf, time::Duration};

use pallas::network::{facades::PeerClient, miniprotocols::Point};
use tokio::{
    sync::{mpsc, oneshot},
    task::JoinHandle,
    time::timeout,
};
use tracing::debug;

use crate::{
    chain_update::ChainUpdate,
    error::{Error, Result},
    mithril_snapshot::MithrilSnapshot,
    multi_era_block_data::MultiEraBlockData,
    network::Network,
    point_or_tip::PointOrTip,
};

/// Default [`Follower`] block buffer size.
const DEFAULT_CHAIN_UPDATE_BUFFER_SIZE: usize = 32;

// Mainnet Defaults.
/// Default Relay to use
const DEFAULT_MAINNET_RELAY: &str = "backbone.cardano.iog.io:3001";
/// Main-net Mithril Signature genesis vkey.
const DEFAULT_MAINNET_MITHRIL_GENESIS_KEY: &str = include_str!("data/mainnet-genesis.vkey");
/// Default Mithril Aggregator to use.
const DEFAULT_MAINNET_MITHRIL_AGGREGATOR: &str =
    "https://aggregator.release-mainnet.api.mithril.network/aggregator";

// Preprod Defaults
/// Default Relay to use
const DEFAULT_PREPROD_RELAY: &str = "preprod-node.play.dev.cardano.org:3001";
/// Preprod network Mithril Signature genesis vkey.
const DEFAULT_PREPROD_MITHRIL_GENESIS_KEY: &str = include_str!("data/preprod-genesis.vkey");
/// Default Mithril Aggregator to use.
const DEFAULT_PREPROD_MITHRIL_AGGREGATOR: &str =
    "https://aggregator.release-preprod.api.mithril.network/aggregator";

// Preview Defaults
/// Default Relay to use
const DEFAULT_PREVIEW_RELAY: &str = "preview-node.play.dev.cardano.org:3001";
/// Preview network Mithril Signature genesis vkey.
const DEFAULT_PREVIEW_MITHRIL_GENESIS_KEY: &str = include_str!("data/preview-genesis.vkey");
/// Default Mithril Aggregator to use.
const DEFAULT_PREVIEW_MITHRIL_AGGREGATOR: &str =
    "https://aggregator.pre-release-preview.api.mithril.network/aggregator";

/// A Follower Connection to the Cardano Network.
#[derive(Clone, Debug)]
pub struct FollowerConfig {
    /// Chain Network
    pub chain: Network,
    /// Relay Node Address
    relay_address: String,
    /// Block buffer size option.
    chain_update_buffer_size: usize,
    /// Where to start following from.
    follow_from: PointOrTip,
    /// Path to the Mithril snapshot the follower should use.
    pub mithril_snapshot_path: Option<PathBuf>,
    /// Address of the Mithril Aggregator to use to find the latest snapshot data to
    /// download.
    pub mithril_aggregator_address: Option<String>,
    /// The Genesis Key needed for a network to do Mithril snapshot validation.
    pub mithril_genesis_key: Option<String>,
    /// Is the mithril snapshot to be transparently updated to latest, in the background.
    pub mithril_update: bool,
    // pub genesis_parameters: GenesisParameters,
}

/// Builder used to create [`FollowerConfig`]s.
#[derive(Clone, Debug)]
pub struct FollowerConfigBuilder(FollowerConfig);

impl FollowerConfigBuilder {
    /// Sets the defaults for a given cardano network.
    /// Each network has a different set of defaults, so no single "default" can apply.
    /// This function is preferred to the `default()` standard function.
    #[must_use]
    pub fn default_for(chain: Network) -> Self {
        match chain {
            Network::Mainnet => Self(FollowerConfig {
                chain,
                relay_address: DEFAULT_MAINNET_RELAY.to_string(),
                chain_update_buffer_size: DEFAULT_CHAIN_UPDATE_BUFFER_SIZE,
                follow_from: PointOrTip::Tip,
                mithril_snapshot_path: None,
                mithril_aggregator_address: Some(DEFAULT_MAINNET_MITHRIL_AGGREGATOR.to_string()),
                mithril_genesis_key: Some(DEFAULT_MAINNET_MITHRIL_GENESIS_KEY.to_string()),
                mithril_update: false,
            }),
            Network::Preview => Self(FollowerConfig {
                chain,
                relay_address: DEFAULT_PREVIEW_RELAY.to_string(),
                chain_update_buffer_size: DEFAULT_CHAIN_UPDATE_BUFFER_SIZE,
                follow_from: PointOrTip::Tip,
                mithril_snapshot_path: None,
                mithril_aggregator_address: Some(DEFAULT_PREVIEW_MITHRIL_AGGREGATOR.to_string()),
                mithril_genesis_key: Some(DEFAULT_PREVIEW_MITHRIL_GENESIS_KEY.to_string()),
                mithril_update: false,
            }),
            Network::Preprod => Self(FollowerConfig {
                chain,
                relay_address: DEFAULT_PREPROD_RELAY.to_string(),
                chain_update_buffer_size: DEFAULT_CHAIN_UPDATE_BUFFER_SIZE,
                follow_from: PointOrTip::Tip,
                mithril_snapshot_path: None,
                mithril_aggregator_address: Some(DEFAULT_PREPROD_MITHRIL_AGGREGATOR.to_string()),
                mithril_genesis_key: Some(DEFAULT_PREPROD_MITHRIL_GENESIS_KEY.to_string()),
                mithril_update: false,
            }),
        }
    }

    /// Sets the size of the chain updates buffer used by the [`Follower`].
    ///
    /// # Arguments
    ///
    /// * `chain_update_buffer_size`: Size of the chain updates buffer.
    #[must_use]
    pub fn chain_update_buffer_size(mut self, block_buffer_size: usize) -> Self {
        self.0.chain_update_buffer_size = block_buffer_size;
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
        self.0.follow_from = from.into();
        self
    }

    /// Sets the path of the Mithril snapshot the [`Follower`] will use.
    ///
    /// # Arguments
    ///
    /// * `path`: Mithril snapshot path.
    /// * `update`: Auto-update this path with the latest mithril snapshot as it changes.
    #[must_use]
    pub fn mithril_snapshot_path(mut self, path: PathBuf, update: bool) -> Self {
        self.0.mithril_snapshot_path = Some(path);
        self.0.mithril_update = update;
        self
    }

    /// Builds a [`FollowerConfig`].
    #[must_use]
    pub fn build(self) -> FollowerConfig {
        self.0
    }
}

/// Try and connect, but if it takes longer then 5 seconds, retry the connection.
/// Retry 5 times before giving up.
async fn retry_connect(
    addr: &str, magic: u64,
) -> std::result::Result<pallas::network::facades::PeerClient, pallas::network::facades::Error> {
    let mut retries = 5;
    loop {
        match timeout(Duration::from_secs(2), PeerClient::connect(addr, magic)).await {
            Ok(peer) => match peer {
                Ok(peer) => return Ok(peer),
                Err(err) => {
                    retries -= 1;
                    if retries == 0 {
                        return Err(err);
                    }
                    debug!("retrying {retries} connect to {addr} : {err:?}");
                },
            },
            Err(error) => {
                retries -= 1;
                if retries == 0 {
                    return Err(pallas::network::facades::Error::ConnectFailure(
                        tokio::io::Error::new(
                            tokio::io::ErrorKind::Other,
                            format!("failed to connect to {addr} : {error}"),
                        ),
                    ));
                }
                debug!("retrying {retries} connect to {addr} : {error:?}");
            },
        }
    }
}

impl FollowerConfig {
    /// Connects the follower to a node, and/or to a mithril snapshot.
    /// Nodes connect using the node-to-node protocol.
    ///
    /// # Arguments
    ///
    /// All arguments come from the configuration.
    ///
    /// # Errors
    ///
    /// Returns Err if the connection could not be established.
    pub async fn connect(self) -> Result<Follower> {
        debug!("Follower Connecting.");

        // Sometimes this takes a really long time.
        // Other times it's neigh on instantaneous.
        // Time out if its too long, and try again.
        // Wrap the future with a `Timeout` set to expire in 5 seconds (and try 5 times).
        let mut client = retry_connect(&self.relay_address, self.chain.into())
            .await
            .map_err(Error::Client)?;

        debug!("Follower client created");

        let Some(follow_from) =
            set_client_read_pointer(&mut client, self.follow_from.clone()).await?
        else {
            return Err(Error::SetReadPointer);
        };

        debug!("Client Read Pointer set OK : {follow_from:?}");

        //MithrilSnapshot::init(self.clone()).await?;

        debug!("Mithril Snapshot initialized.");

        let (task_request_tx, chain_update_rx, task_join_handle) =
            task::FollowTask::spawn(client, self.clone(), follow_from);

        debug!("Follower Task started.");

        Ok(Follower {
            connection_cfg: self.clone(),
            chain_update_rx,
            follow_task_request_tx: task_request_tx,
            follow_task_join_handle: task_join_handle,
        })
    }
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
            std::task::Poll::Ready(res) => match res {
                Ok(res) => std::task::Poll::Ready(res),
                Err(_) => std::task::Poll::Ready(Err(Error::Internal)),
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
            std::task::Poll::Ready(res) => match res {
                Ok(res) => std::task::Poll::Ready(res),
                Err(_) => std::task::Poll::Ready(Err(Error::Internal)),
            },
            std::task::Poll::Pending => std::task::Poll::Pending,
        }
    }
}

/// Cardano chain follower.
#[derive(Debug)]
pub struct Follower {
    /// Client connection information.
    ///
    /// This is used to open more connections when needed.
    connection_cfg: FollowerConfig,
    /// Chain update receiver.
    chain_update_rx: mpsc::Receiver<Result<ChainUpdate>>,
    /// Follow task request sender.
    follow_task_request_tx: mpsc::Sender<task::SetReadPointerRequest>,
    /// Follow task thread join handle.
    follow_task_join_handle: JoinHandle<()>,
}

impl Follower {
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
    where
        P: Into<PointOrTip>,
    {
        let at = at.into();

        let relay_address = self.connection_cfg.relay_address.clone();
        let network = self.connection_cfg.chain;

        let join_handle = tokio::spawn(async move {
            let mut client = PeerClient::connect(relay_address, network.into())
                .await
                .map_err(Error::Client)?;

            match at {
                PointOrTip::Tip => {
                    let point = resolve_tip(&mut client).await?;
                    read_block_from_network(&mut client, point).await
                },

                PointOrTip::Point(point) => {
                    let snapshot_res = MithrilSnapshot::try_read_block(network, &point)
                        .ok()
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
    where
        P: Into<PointOrTip>,
    {
        let to = to.into();

        let relay_address = self.connection_cfg.relay_address.clone();
        let network = self.connection_cfg.chain;

        let join_handle = tokio::spawn(async move {
            let mut client = PeerClient::connect(relay_address, network.into())
                .await
                .map_err(Error::Client)?;

            match to {
                PointOrTip::Tip => {
                    let to_point = resolve_tip(&mut client).await?;
                    read_block_range_from_network(&mut client, from, to_point).await
                },
                PointOrTip::Point(to) => {
                    let snapshot_res = MithrilSnapshot::try_read_block_range(network, &from, &to)
                        .ok()
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
    use tracing::debug;

    use super::{set_client_read_pointer, ChainUpdate};
    use crate::{
        error::{Error, Result},
        mithril_snapshot::MithrilSnapshot,
        multi_era_block_data::MultiEraBlockData,
        network::Network,
        point_or_tip::PointOrTip,
        FollowerConfig,
    };

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
        connection_cfg: FollowerConfig,
        /// Request receiver.
        request_rx: mpsc::Receiver<SetReadPointerRequest>,
        /// Chain update sender.
        chain_update_tx: mpsc::Sender<Result<ChainUpdate>>,
    }

    impl FollowTask {
        /// Spawn a follow task.
        pub(super) fn spawn(
            client: PeerClient, connection_cfg: FollowerConfig, follow_from: Point,
        ) -> (
            mpsc::Sender<SetReadPointerRequest>,
            mpsc::Receiver<Result<ChainUpdate>>,
            tokio::task::JoinHandle<()>,
        ) {
            let (request_tx, request_rx) = mpsc::channel(1);
            let (chain_update_tx, chain_update_rx) =
                mpsc::channel(connection_cfg.chain_update_buffer_size);

            let this = Self {
                connection_cfg,
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
                self.connection_cfg.chain,
                self.chain_update_tx.clone(),
                from,
            );
            tokio::pin!(fetch_chain_updates_fut);

            loop {
                tokio::select! {
                    Some(SetReadPointerRequest { at, response_tx }) = self.request_rx.recv() => {
                        let res = PeerClient::connect(&self.connection_cfg.relay_address, self.connection_cfg.chain.into())
                            .await;

                        let Ok(mut client) = res else {
                            drop(response_tx.send(Err(Error::SetReadPointer)));
                            continue;
                        };

                        match set_client_read_pointer(&mut client, at).await {
                            Ok(Some(from)) => {
                                fetch_chain_updates_fut.set(Self::fetch_chain_updates(
                                    client,
                                    self.connection_cfg.chain,
                                    self.chain_update_tx.clone(),
                                    from.clone(),
                                ));

                                drop(response_tx.send(Ok(Some(from))));
                            }
                            Ok(None) => {
                                drop(response_tx.send(Ok(None)));
                            }
                            Err(_) => {
                                drop(response_tx.send(Err(Error::SetReadPointer)));
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
            mut client: PeerClient, network: Network,
            chain_update_tx: mpsc::Sender<Result<ChainUpdate>>, from: Point,
        ) {
            let mut current_point = from;

            let set_to_snapshot =
                MithrilSnapshot::try_read_blocks_from_point(network, &current_point);

            if let Some(iter) = set_to_snapshot {
                let mut last_recv_from_snapshot = false;

                for result in iter {
                    let mut fallback = false;

                    if let Ok(raw_block_data) = result {
                        let block_data = MultiEraBlockData::new(raw_block_data);

                        match block_data.decode() {
                            Ok(block) => {
                                current_point =
                                    Point::Specific(block.slot(), block.hash().to_vec());

                                if chain_update_tx
                                    .send(Ok(ChainUpdate::ImmutableBlock(block_data)))
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
                                drop(chain_update_tx.send(Err(Error::SetReadPointer)).await);
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
        async fn next_from_client(client: &mut PeerClient) -> Result<Option<ChainUpdate>> {
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
                chainsync::NextResponse::RollForward(header, tip) => {
                    // Note: Tip is poorly documented.
                    // It is a tuple with the following structure:
                    // ((Slot#, BlockHash), Block# ).
                    // We can find if we are AT tip by comparing the current block Point with the tip Point.
                    // We can estimate how far behind we are (in blocks) by subtracting current block
                    // height and the tip block height.
                    let decoded_header = MultiEraHeader::decode(
                        header.variant,
                        header.byron_prefix.map(|p| p.0),
                        &header.cbor,
                    )
                    .map_err(Error::Codec)?;

                    let point =
                        Point::Specific(decoded_header.slot(), decoded_header.hash().to_vec());

                    // See if this block is the current nodes TIP.
                    let at_tip = point == tip.0;
                    debug!("At Tip? {point:?} == {tip:?}");

                    tracing::trace!(point = ?point, "Fetching roll forward block data");
                    let block_data = client
                        .blockfetch()
                        .fetch_single(point)
                        .await
                        .map_err(Error::Blockfetch)?;

                    let update = if at_tip {
                        // Then we are at the tip of the blockchain.
                        ChainUpdate::BlockTip(MultiEraBlockData::new(block_data))
                    } else {
                        ChainUpdate::Block(MultiEraBlockData::new(block_data))
                    };

                    Ok(Some(update))
                },
                chainsync::NextResponse::RollBackward(point, _tip) => {
                    tracing::trace!(point = ?point, "Fetching roll backward block data");
                    let block_data = client
                        .blockfetch()
                        .fetch_single(point)
                        .await
                        .map_err(Error::Blockfetch)?;

                    Ok(Some(ChainUpdate::Rollback(MultiEraBlockData::new(
                        block_data,
                    ))))
                },
                chainsync::NextResponse::Await => Ok(None),
            }
        }

        /// Sends the next chain update through the follower's chain update channel.
        async fn send_next_chain_update(
            client: &mut PeerClient, chain_update_tx: mpsc::Sender<Result<ChainUpdate>>,
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
    Ok(MultiEraBlockData::new(block_data))
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
        .map(MultiEraBlockData::new)
        .collect();

    tracing::trace!(from_slot, to_slot, "Block range read from n2n");

    Ok(data_vec)
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use test_log::test;
    use tokio::time::sleep;
    use tracing::debug;

    use crate::{
        chain_update::ChainUpdate::{
            Block, BlockTip, ImmutableBlock, ImmutableBlockRollback, Rollback,
        },
        network::Network,
        point_or_tip::PointOrTip,
        FollowerConfigBuilder, Point,
    };

    #[ignore]
    #[test(tokio::test)]
    // Development only test, not for CI.
    async fn test_follow_preprod() {
        tracing::dispatcher::get_default(|x| debug!("{x:?}"));

        let follower = FollowerConfigBuilder::default_for(Network::Preprod)
            .follow_from(PointOrTip::Point(Point::Origin))
            .mithril_snapshot_path("/tmp/mithril/preprod".into(), true)
            .build();

        debug!("{follower:?}");

        let connection = follower.connect().await;

        debug!("{connection:?}");

        assert!(connection.is_ok());
        #[allow(clippy::unwrap_used)]
        let mut connection = connection.unwrap();

        let mut consecutive_errors = 0;
        let mut immutable_blocks = 0;
        let mut live_blocks = 0;
        let mut live_block_tips = 0;
        let mut rollbacks = 0;

        while consecutive_errors < 100 {
            let block = connection.next().await;
            match block {
                Err(err) => {
                    debug!("Block Error ({consecutive_errors}): {err:?}");
                    sleep(Duration::from_secs(30)).await;
                    consecutive_errors += 1;
                },
                Ok(data) => {
                    let data_msg = format!("{data}");

                    match &data {
                        ImmutableBlock(_) => {
                            if immutable_blocks % 10_000 == 0 {
                                debug!("Immutable Block {immutable_blocks} : {data_msg}");
                            }
                            immutable_blocks += 1;
                        },
                        ImmutableBlockRollback(_) => {
                            debug!("Immutable Block Rollback {immutable_blocks} : {data_msg}");
                            immutable_blocks += 1;
                        },
                        Block(_) => {
                            if live_blocks == 0 {
                                debug!("Total Immutable Blocks = {immutable_blocks}");
                            }
                            live_blocks += 1;
                            debug!("Live Block {live_blocks}/T#{live_block_tips}/R#{rollbacks} : {data_msg}");
                        },
                        BlockTip(_) => {
                            if live_blocks == 0 {
                                debug!("Total Immutable Blocks = {immutable_blocks}");
                            }
                            live_blocks += 1;
                            live_block_tips += 1;
                            debug!("Live Block Tip {live_blocks}/T#{live_block_tips}/R#{rollbacks} : {data_msg}");
                        },
                        Rollback(_) => {
                            if live_blocks == 0 {
                                debug!("Total Immutable Blocks = {immutable_blocks}");
                            }
                            live_blocks += 1;
                            rollbacks += 1;
                            debug!("Live Block Rollback {live_blocks}/{live_block_tips}/{rollbacks} : {data_msg}");
                        },
                    }
                },
            }
        }
    }
}
