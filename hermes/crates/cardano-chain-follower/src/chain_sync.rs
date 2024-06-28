//! Sync from the chain to an in-memory buffer.
//!
//! All iteration of the chain is done through this buffer or a mithril snapshot.
//! Consumers of this library do not talk to the node directly.

use std::time::Duration;

use anyhow::Context;
use pallas::{
    ledger::traverse::MultiEraHeader,
    network::{
        facades::PeerClient,
        miniprotocols::{
            chainsync::{self, ClientError, Tip},
            Point,
        },
    },
};
use tokio::{
    spawn,
    sync::{broadcast, mpsc},
    time::{sleep, timeout},
};
use tracing::{debug, error};

use crate::{
    chain_sync_live_chains::{
        get_fill_to_point, get_live_block_at, latest_live_point, live_chain_insert,
        live_chain_length, purge_latest_live_point, purge_live_chain, PurgeType,
    },
    chain_sync_ready::{get_chain_update_tx_queue, wait_for_sync_ready, SyncReadyWaiter},
    chain_update,
    error::{Error, Result},
    mithril_snapshot::MithrilSnapshot,
    mithril_snapshot_config::MithrilUpdateMessage,
    multi_era_block_data::UNKNOWN_POINT,
    ChainSyncConfig, ChainUpdate, MultiEraBlock, Network, PointOrTip,
};

/// The maximum number of seconds we wait for a node to connect.
const MAX_NODE_CONNECT_TIME_SECS: u64 = 2;

/// The maximum number of times we wait for a nodeChainUpdate to connect.
/// Currently set to never give up.
const MAX_NODE_CONNECT_RETRIES: u64 = 5;

/// Try and connect to a node, in a robust and quick way.
///
/// If it takes longer then 5 seconds, retry the connection.
/// Retry 5 times before giving up.
async fn retry_connect(
    addr: &str, magic: u64,
) -> std::result::Result<PeerClient, pallas::network::facades::Error> {
    let mut retries = MAX_NODE_CONNECT_RETRIES;
    loop {
        match timeout(
            Duration::from_secs(MAX_NODE_CONNECT_TIME_SECS),
            PeerClient::connect(addr, magic),
        )
        .await
        {
            Ok(peer) => {
                match peer {
                    Ok(peer) => return Ok(peer),
                    Err(err) => {
                        retries -= 1;
                        if retries == 0 {
                            return Err(err);
                        }
                        debug!("retrying {retries} connect to {addr} : {err:?}");
                    },
                }
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

/// Set the Client Read Pointer for this connection with the Node
async fn set_client_read_pointer(client: &mut PeerClient, at: PointOrTip) -> Result<Point> {
    match at {
        PointOrTip::Point(Point::Origin) => {
            client
                .chainsync()
                .intersect_origin()
                .await
                .map_err(Error::Chainsync)
        },
        PointOrTip::Tip => {
            client
                .chainsync()
                .intersect_tip()
                .await
                .map_err(Error::Chainsync)
        },
        PointOrTip::Point(p @ Point::Specific(..)) => {
            match client.chainsync().find_intersect(vec![p]).await {
                Ok((point, _)) => {
                    match point {
                        Some(point) => Ok(point),
                        None => Err(Error::Chainsync(ClientError::IntersectionNotFound)),
                    }
                },
                Err(error) => Err(Error::Chainsync(error)),
            }
        },
    }
}

/// Resynchronize to the live tip in memory.
async fn resync_live_tip(client: &mut PeerClient, chain: Network) -> Result<Point> {
    let tip = latest_live_point(chain);

    set_client_read_pointer(client, tip).await
}

/// Sand a chain update to any subscribers that are listening.
fn send_update(
    chain: Network, update_sender: &Option<broadcast::Sender<ChainUpdate>>, point: &PointOrTip,
    update: ChainUpdate,
) {
    if let Some(update_sender) = update_sender {
        if let Err(error) = update_sender.send(update) {
            error!(
                chain = chain.to_string(),
                point = format!("{:?}", point),
                "Failed to broadcast the Update : {error}"
            );
        }
    }
}

/// Process a rollback.
fn process_rollback(chain: Network, point: Point, tip: &Tip) -> anyhow::Result<Option<Point>> {
    debug!("RollBackward: {:?} {:?}", point, tip);

    purge_live_chain(chain, &point, PurgeType::Newest);

    // If we have ANY live blocks, then we MUST have the rollback to block, or its a fatal
    // sync error.
    if live_chain_length(chain) > 0 {
        let rollback_block = get_live_block_at(chain, &point);
        if rollback_block.is_none() {
            error!(
                chain = chain.to_string(),
                point = format!("{:?}", point),
                tip = format!("{:?}", tip),
                "No live block at rollback point, this is a fatal sync error"
            );
            return Err(Error::LiveSync("No live block at rollback point".to_string()).into());
        }
    }

    // Next block we receive is a rollback.
    Ok(Some(point))
}

/// Follows the chain until there is an error.
/// If this returns it can be assumed the client is disconnected.
///
/// We take ownership of the client because of that.
async fn follow_chain(peer: &mut PeerClient, chain: Network) -> anyhow::Result<()> {
    let mut update_sender = get_chain_update_tx_queue(chain).await;
    let mut block_is_rollback: Option<Point> = None;

    let mut previous_point = UNKNOWN_POINT;

    loop {
        // debug!("Waiting for data from Cardano Peer Node:");

        // We can't get an update sender UNTIL we have released the sync lock.
        if update_sender.is_none() {
            update_sender = get_chain_update_tx_queue(chain).await;
        }

        // Check what response type we need to process.
        let response = match peer.chainsync().state() {
            chainsync::State::CanAwait => peer.chainsync().recv_while_can_await().await,
            chainsync::State::MustReply => peer.chainsync().recv_while_must_reply().await,
            _ => peer.chainsync().request_next().await,
        }
        .with_context(|| "Error while receiving block data from peer")?;

        match response {
            chainsync::NextResponse::RollForward(header, tip) => {
                // Note: Tip is poorly documented.
                // It is a tuple with the following structure:
                // ((Slot#, BlockHash), Block# ).
                // We can find if we are AT tip by comparing the current block Point with the tip
                // Point. We can estimate how far behind we are (in blocks) by
                // subtracting current block height and the tip block height.
                // IF the TIP is <= the current block height THEN we are at tip.
                let decoded_header = MultiEraHeader::decode(
                    header.variant,
                    header.byron_prefix.map(|p| p.0),
                    &header.cbor,
                )
                .with_context(|| "Decoding Block Header")?;

                let point = Point::Specific(decoded_header.slot(), decoded_header.hash().to_vec());

                debug!("RollForward: {:?} {:?}", point, tip);

                let block_data = peer
                    .blockfetch()
                    .fetch_single(point.clone())
                    .await
                    .with_context(|| "Fetching block data")?;

                let live_block_data =
                    MultiEraBlock::new(chain, block_data, &previous_point, false)?;

                // We can't store this block because we don't know the previous one so the chain
                // would break, so just use it for previous.
                if previous_point == UNKNOWN_POINT {
                    previous_point = live_block_data.point();
                    // Nothing else we can do with the first block when we don't know the previous
                    // one.
                    continue;
                }
                // Add the live block to the head of the live chain
                live_chain_insert(chain, live_block_data.clone());
                previous_point = point.clone();

                let reported_tip = PointOrTip::Point(tip.0);
                let block_point = PointOrTip::Point(point.clone());

                let mut update_type = chain_update::Kind::Block;

                // IF its a rollback block, report it as such.
                if let Some(rollback) = block_is_rollback.take() {
                    // If the live chain is empty, rollback doesn't make sense.
                    if live_chain_length(chain) > 0 {
                        // Can't rely on TIP so we need to check the block itself to see if we are
                        // intact.
                        debug!("RollBack: {:?}", rollback);
                        debug!("Chain Length: {:?}", live_chain_length(chain));
                        debug!("Previous Hash: {:?}", decoded_header.previous_hash());
                        if let Point::Specific(_slot, hash) = rollback {
                            if let Some(previous_hash) = decoded_header.previous_hash() {
                                if hash != previous_hash.as_ref() {
                                    return Err(Error::LiveSync(
                                        "Rollback block previous hash does not match".to_string(),
                                    )
                                    .into());
                                }
                            } else {
                                return Err(Error::LiveSync(
                                    "Rollback block previous hash is missing.".to_string(),
                                )
                                .into());
                            }
                        } else {
                            return Err(
                                Error::LiveSync("Invalid Rollback block".to_string()).into()
                            );
                        }

                        update_type = chain_update::Kind::Rollback;
                    }
                }

                let update =
                    ChainUpdate::new(update_type, reported_tip <= block_point, live_block_data);
                send_update(chain, &update_sender, &block_point, update);
            },
            chainsync::NextResponse::RollBackward(point, tip) => {
                block_is_rollback = process_rollback(chain, point, &tip)?;
            },
            chainsync::NextResponse::Await => {
                // debug!("Peer Node says: Await");
            },
        }
    }
}

/// Do not return until we have a connection to the peer.
async fn persistent_reconnect(addr: &str, chain: Network) -> PeerClient {
    loop {
        // We never have a connection if we end up around the loop, so make a new one.
        match retry_connect(addr, chain.into()).await {
            Ok(peer) => return peer,
            Err(error) => {
                error!(
                    "Chain Sync for: {} from   {}  : Failed to connect to relay: {}",
                    chain, addr, error,
                );
            },
        };
    }
}

/// Backfill the live chain, based on the Mithril Sync updates.
/// This does NOT return until the live chain has been backfilled from the end of mithril
/// to the current synced tip blocks.
///
/// This only needs to be done once per chain connection.
async fn live_sync_backfill(
    cfg: &ChainSyncConfig, update: &MithrilUpdateMessage,
) -> anyhow::Result<()> {
    let fill_to = get_fill_to_point(cfg.chain).await;
    let range = (update.tip.clone(), fill_to);
    let mut previous_point = update.previous.clone();

    let range_msg = format!("{range:?}");

    let mut peer = persistent_reconnect(&cfg.relay_address, cfg.chain).await;

    // Request the range of blocks from the Peer.
    peer.blockfetch()
        .request_range(range)
        .await
        .with_context(|| "Requesting Block Range")?;

    while let Some(block_data) = peer.blockfetch().recv_while_streaming().await? {
        let block = MultiEraBlock::new(cfg.chain, block_data, &previous_point, false)
            .with_context(|| {
                format!(
                    "Failed to decode block data. previous: {previous_point:?}, range: {range_msg}"
                )
            })?;

        previous_point = block.point();
        live_chain_insert(cfg.chain, block);
        // debug!("Backfilled Block: {}", slot);
    }

    debug!("Backfilled Range OK: {}", range_msg);

    Ok(())
}

/// Backfill and Purge the live chain, based on the Mithril Sync updates.
async fn live_sync_backfill_and_purge(
    cfg: ChainSyncConfig, mut rx: mpsc::Receiver<MithrilUpdateMessage>,
    mut sync_ready: SyncReadyWaiter,
) {
    let Some(update) = rx.recv().await else {
        error!("Mithril Sync Failed, can not continue chain sync either.");
        return;
    };

    debug!(
        "Before Backfill: Size of the Live Chain is: {} Blocks",
        live_chain_length(cfg.chain)
    );

    // Wait for first Mithril Update advice, which triggers a BACKFILL of the Live Data.
    debug!("Mithril Tip has advanced to: {update:?} : BACKFILL");
    while let Err(error) = live_sync_backfill(&cfg, &update).await {
        error!("Mithril Backfill Sync Failed: {}", error);
        sleep(Duration::from_secs(10)).await;
    }

    debug!(
        "After Backfill: Size of the Live Chain is: {} Blocks",
        live_chain_length(cfg.chain)
    );

    // Once Backfill is completed OK we can use the Blockchain data for Syncing and Querying
    sync_ready.signal();

    let mut update_sender = get_chain_update_tx_queue(cfg.chain).await;

    loop {
        let Some(update) = rx.recv().await else {
            error!("Mithril Sync Failed, can not continue chain sync either.");
            return;
        };

        // We can't get an update sender until the sync is released.
        if update_sender.is_none() {
            update_sender = get_chain_update_tx_queue(cfg.chain).await;
        }

        debug!("Mithril Tip has advanced to: {update:?} : PURGE NEEDED");

        purge_live_chain(cfg.chain, &update.tip, PurgeType::Oldest);

        debug!(
            "After Purge: Size of the Live Chain is: {} Blocks",
            live_chain_length(cfg.chain)
        );

        if let Some(block_data) = MithrilSnapshot::new(cfg.chain).read_block_at(&update.tip) {
            let update_point = PointOrTip::Point(block_data.point());
            // Get Immutable block that represents this point
            let update = ChainUpdate::new(
                chain_update::Kind::ImmutableBlockRollForward,
                true, // Tip of the immutable blockchain
                block_data,
            );
            send_update(cfg.chain, &update_sender, &update_point, update);
        } else {
            error!(
                chain = cfg.chain.to_string(),
                point = format!("{:?}", update.tip),
                "Immutable Chain update, but block not found."
            );
        }
    }

    // TODO: If the mithril sync dies, sleep for a bit and make sure the live chain
    // doesn't grow indefinitely.
    // We COULD move the spawn of mithril following into here, and if the rx dies, kill
    // that task, and restart it.
    // In reality, the mithril sync should never die and drop the queue.
}

/// Handle the background downloading of Mithril snapshots for a given network.
/// Note: There can ONLY be at most three of these running at any one time.
/// This is because there can ONLY be one snapshot for each of the three known Cardano
/// networks.
/// # Arguments
///
/// * `network` - The network type for the client to connect to.
/// * `aggregator_url` - A reference to the URL of an aggregator that can be used to
///   create the client.
/// * `genesis_vkey` - The genesis verification key, which is needed to authenticate with
///   the server.
///
/// # Returns
///
/// This does not return, it is a background task.
pub(crate) async fn chain_sync(cfg: ChainSyncConfig, rx: mpsc::Receiver<MithrilUpdateMessage>) {
    debug!(
        "Chain Sync for: {} from {} : Starting",
        cfg.chain, cfg.relay_address,
    );

    // Start the SYNC_READY unlock task.
    let sync_waiter = wait_for_sync_ready(cfg.chain);

    let backfill_cfg = cfg.clone();

    // Start the Live chain backfill task.
    let _backfill_join_handle = spawn(async move {
        live_sync_backfill_and_purge(backfill_cfg.clone(), rx, sync_waiter).await;
    });

    loop {
        // We never have a connection if we end up around the loop, so make a new one.
        let mut peer = persistent_reconnect(&cfg.relay_address, cfg.chain).await;

        if let Err(error) = resync_live_tip(&mut peer, cfg.chain).await {
            // If we fail to resync the tip, then we should stop trying to sync.
            // We'll try again next time.
            error!(
                "Cardano Client {} failed to resync Tip: {}",
                cfg.relay_address, error
            );

            // Couldn't sync to last known block, so purge it.
            purge_latest_live_point(cfg.chain);
            continue;
        };

        // Note: This can ONLY return with an error, otherwise it will sync indefinitely.
        if let Err(error) = follow_chain(&mut peer, cfg.chain).await {
            error!(
                "Cardano Client {} failed to follow chain: {}: Reconnecting.",
                cfg.relay_address, error
            );
            continue;
        }
    }
}
