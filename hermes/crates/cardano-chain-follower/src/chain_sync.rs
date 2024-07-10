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
        miniprotocols::chainsync::{self, HeaderContent, Tip},
    },
};
use tokio::{
    spawn,
    sync::mpsc,
    time::{sleep, timeout},
};
use tracing::{debug, error};

use crate::{
    chain_sync_live_chains::{
        get_fill_to_point, get_intersect_points, get_live_block, get_live_head_point, get_peer_tip,
        live_chain_add_block_to_tip, live_chain_backfill, live_chain_length, purge_live_chain,
    },
    chain_sync_ready::{
        get_chain_update_tx_queue, notify_follower, wait_for_sync_ready, SyncReadyWaiter,
    },
    chain_update,
    error::{Error, Result},
    mithril_snapshot_config::MithrilUpdateMessage,
    mithril_snapshot_data::latest_mithril_snapshot_id,
    point::{TIP_POINT, UNKNOWN_POINT},
    stats, ChainSyncConfig, MultiEraBlock, Network, Point, ORIGIN_POINT,
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

/// Purge the live chain, and intersect with TIP.
async fn purge_and_intersect_tip(client: &mut PeerClient, chain: Network) -> Result<Point> {
    if let Err(error) = purge_live_chain(chain, &TIP_POINT) {
        // Shouldn't happen.
        error!("failed to purge live chain: {error}");
    }

    client
        .chainsync()
        .intersect_tip()
        .await
        .map_err(Error::Chainsync)
        .map(std::convert::Into::into)
}

/// Resynchronize to the live tip in memory.
async fn resync_live_tip(client: &mut PeerClient, chain: Network) -> Result<Point> {
    let sync_points = get_intersect_points(chain);
    if sync_points.is_empty() {
        return purge_and_intersect_tip(client, chain).await;
    }

    let sync_to_point = match client.chainsync().find_intersect(sync_points).await {
        Ok((Some(point), _)) => point.into(),
        Ok((None, _)) => {
            // No intersection found, so purge live chain and re-sync it.
            return purge_and_intersect_tip(client, chain).await;
        },
        Err(error) => return Err(Error::Chainsync(error)),
    };

    Ok(sync_to_point)
}

/// Fetch a single block from the Peer, and Decode it.
async fn fetch_block_from_peer(
    peer: &mut PeerClient, chain: Network, point: Point, previous_point: Point, fork_count: u64,
) -> anyhow::Result<MultiEraBlock> {
    let block_data = peer
        .blockfetch()
        .fetch_single(point.clone().into())
        .await
        .with_context(|| "Fetching block data")?;

    debug!("{chain}, {previous_point}, {fork_count}");
    let live_block_data = MultiEraBlock::new(chain, block_data, &previous_point, fork_count)?;

    Ok(live_block_data)
}

/// Process a rollback.
///
/// Fetch the rollback block, and try and insert it into the live-chain.
/// If its a real rollback, it will purge the chain ahead of the block automatically.
async fn process_rollback_actual(
    peer: &mut PeerClient, chain: Network, point: Point, tip: &Tip, fork_count: &mut u64,
) -> anyhow::Result<Point> {
    debug!("RollBackward: {:?} {:?}", point, tip);

    // Check if the block is in the live chain, if it is, re-add it, which auto-purges the rest of live chain tip.
    // And increments the fork count.
    if let Some(mut block) = get_live_block(chain, &point, 0, true) {
        // Even though we are re-adding the known block, increase the fork count.
        block.set_fork(*fork_count);
        live_chain_add_block_to_tip(chain, block, fork_count, tip.0.clone().into())?;
        return Ok(point);
    }

    // If the block is NOT in the chain, fetch it, and insert it, which will automatically find the correct place to
    // insert it, and purge the old tip blocks.

    // We don't know what or if there is a previous block, so probe for it.
    // Fizzy search for the block immediately preceding the block we will fetch.
    // In case we don;t have a previous point on the live chain, it might be the tip of the mithril chain, so get that.
    let previous_block = get_live_block(chain, &point, -1, false);
    let previous_point = if let Some(previous_block) = previous_block {
        let previous = previous_block.previous();
        debug!("Previous block: {:?}", previous);
        if previous == ORIGIN_POINT {
            latest_mithril_snapshot_id(chain).tip()
        } else {
            previous
        }
    } else {
        debug!("Using Mithril Tip as rollback previous point.");
        latest_mithril_snapshot_id(chain).tip()
    };
    debug!("Previous point: {:?}", previous_point);
    let block =
        fetch_block_from_peer(peer, chain, point.clone(), previous_point, *fork_count).await?;
    live_chain_add_block_to_tip(chain, block, fork_count, tip.0.clone().into())?;

    // Next block we receive is a rollback.
    Ok(point)
}

/// Process a rollback detected from the peer.
async fn process_rollback(
    peer: &mut PeerClient, chain: Network, point: Point, tip: &Tip, previous_point: &Point,
    fork_count: &mut u64,
) -> anyhow::Result<Point> {
    let rollback_slot = point.slot_or_default();
    let head_slot = previous_point.slot_or_default();
    debug!("Head slot: {}", head_slot);
    debug!("Rollback slot: {}", rollback_slot);
    let slot_rollback_size = if head_slot > rollback_slot {
        head_slot - rollback_slot
    } else {
        0
    };

    // We actually do the work here...
    let response = process_rollback_actual(peer, chain, point, tip, fork_count).await?;

    // We never really know how many blocks are rolled back when advised by the peer, but we can work out how many slots.
    // This function wraps the real work, so we can properly record the stats when the rollback is complete.
    // Even if it errors.
    stats::rollback(chain, stats::RollbackType::Peer, slot_rollback_size);

    Ok(response)
}

/// Process a rollback detected from the peer.
async fn process_next_block(
    peer: &mut PeerClient, chain: Network, header: HeaderContent, tip: &Tip,
    previous_point: &Point, fork_count: &mut u64,
) -> anyhow::Result<Point> {
    // Decode the Header of the block so we know what to fetch.
    let decoded_header = MultiEraHeader::decode(
        header.variant,
        header.byron_prefix.map(|p| p.0),
        &header.cbor,
    )
    .with_context(|| "Decoding Block Header")?;

    let block_point = Point::new(decoded_header.slot(), decoded_header.hash().to_vec());

    debug!("RollForward: {block_point:?} {tip:?}");

    let block = fetch_block_from_peer(
        peer,
        chain,
        block_point.clone(),
        previous_point.clone(),
        *fork_count,
    )
    .await?;

    let block_point = block.point();

    // We can't store this block because we don't know the previous one so the chain
    // would break, so just use it for previous.
    if *previous_point == UNKNOWN_POINT {
        // Nothing else we can do with the first block when we don't know the previous
        // one.  Just return it's point.
        debug!("Not storing the block, because we did not know the previous point.");
    } else {
        live_chain_add_block_to_tip(chain, block, fork_count, tip.0.clone().into())?;
    }

    Ok(block_point)
}

/// Follows the chain until there is an error.
/// If this returns it can be assumed the client is disconnected.
///
/// We take ownership of the client because of that.
async fn follow_chain(
    peer: &mut PeerClient, chain: Network, fork_count: &mut u64,
) -> anyhow::Result<()> {
    let mut update_sender = get_chain_update_tx_queue(chain).await;
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
                previous_point =
                    process_next_block(peer, chain, header, &tip, &previous_point, fork_count)
                        .await?;

                // This update is just for followers to know to look again at their live chains for new data.
                notify_follower(chain, &update_sender, &chain_update::Kind::Block);
            },
            chainsync::NextResponse::RollBackward(point, tip) => {
                previous_point =
                    process_rollback(peer, chain, point.into(), &tip, &previous_point, fork_count)
                        .await?;
                // This update is just for followers to know to look again at their live chains for new data.
                notify_follower(chain, &update_sender, &chain_update::Kind::Rollback);
            },
            chainsync::NextResponse::Await => {
                // debug!("Peer Node says: Await");
            },
        }
    }
}

/// How long we wait before trying to reconnect to a peer when it totally fails our attempts.
const PEER_FAILURE_RECONNECT_DELAY: Duration = Duration::from_secs(10);

/// Do not return until we have a connection to the peer.
async fn persistent_reconnect(addr: &str, chain: Network) -> PeerClient {
    // Not yet connected to the peer.
    stats::peer_connected(chain, false, addr);

    loop {
        // We never have a connection if we end up around the loop, so make a new one.
        match retry_connect(addr, chain.into()).await {
            Ok(peer) => {
                // Successfully connected to the peer.
                stats::peer_connected(chain, true, addr);

                return peer;
            },
            Err(error) => {
                error!(
                    "Chain Sync for: {} from   {}  : Failed to connect to relay: {}",
                    chain, addr, error,
                );

                // Wait a bit before trying again.
                tokio::time::sleep(PEER_FAILURE_RECONNECT_DELAY).await;
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
    stats::backfill_started(cfg.chain);

    let (fill_to, _oldest_fork) = get_fill_to_point(cfg.chain).await;
    let range = (update.tip.clone().into(), fill_to.clone().into());
    let mut previous_point = update.previous.clone();

    let range_msg = format!("{range:?}");

    let mut peer = persistent_reconnect(&cfg.relay_address, cfg.chain).await;

    // Request the range of blocks from the Peer.
    peer.blockfetch()
        .request_range(range)
        .await
        .with_context(|| "Requesting Block Range")?;

    let mut backfill_blocks = Vec::<MultiEraBlock>::new();

    while let Some(block_data) = peer.blockfetch().recv_while_streaming().await? {
        // Backfilled blocks get placed in the oldest fork currently on the live-chain.
        let block =
            MultiEraBlock::new(cfg.chain, block_data, &previous_point, 1).with_context(|| {
                format!(
                    "Failed to decode block data. previous: {previous_point:?}, range: {range_msg}"
                )
            })?;

        // Check we get the first block in the range properly.
        if backfill_blocks.is_empty() && !block.point().strict_eq(&update.tip) {
            return Err(Error::BackfillSync(format!(
                "First Block is invalid: Block {:?} != Range Start {:?}.",
                block.point(),
                update.tip
            ))
            .into());
        }

        previous_point = block.point();

        backfill_blocks.push(block);
    }

    // Check we get the last block in the range properly.
    if backfill_blocks.is_empty() || !previous_point.strict_eq(&fill_to) {
        return Err(Error::BackfillSync(format!(
            "Last Block is invalid. Block {previous_point:?} != Range End {fill_to:?}"
        ))
        .into());
    }

    // Report how many backfill blocks we received.
    let backfill_size = backfill_blocks.len() as u64;

    // Try and backfill, if anything doesn't work, or the chain integrity would break, fail.
    live_chain_backfill(cfg.chain, &backfill_blocks)?;

    stats::backfill_ended(cfg.chain, backfill_size);

    debug!("Backfilled Range OK: {}", range_msg);

    Ok(())
}

/// Backfill and Purge the live chain, based on the Mithril Sync updates.
async fn live_sync_backfill_and_purge(
    cfg: ChainSyncConfig, mut rx: mpsc::Receiver<MithrilUpdateMessage>,
    mut sync_ready: SyncReadyWaiter,
) {
    // Wait for first Mithril Update advice, which triggers a BACKFILL of the Live Data.
    let Some(update) = rx.recv().await else {
        error!("Mithril Sync Failed, can not continue chain sync either.");
        return;
    };

    debug!(
        "Before Backfill: Size of the Live Chain is: {} Blocks",
        live_chain_length(cfg.chain)
    );

    let live_chain_head: Point;

    loop {
        // We will re-attempt backfill, until its successful.
        // Backfill is atomic, it either fully works, or none of the live-chain is changed.
        debug!("Mithril Tip has advanced to: {update:?} : BACKFILL");
        while let Err(error) = live_sync_backfill(&cfg, &update).await {
            error!("Mithril Backfill Sync Failed: {}", error);
            sleep(Duration::from_secs(10)).await;
        }

        if let Some(head_point) = get_live_head_point(cfg.chain) {
            live_chain_head = head_point;
            break;
        }
    }

    stats::new_mithril_update(
        cfg.chain,
        update.tip.slot_or_default(),
        live_chain_length(cfg.chain) as u64,
        live_chain_head.slot_or_default(),
    );

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

        let update_point: Point = update.tip.clone();

        if let Err(error) = purge_live_chain(cfg.chain, &update_point) {
            // This should actually never happen.
            error!("Mithril Purge Failed: {}", error);
        }

        debug!(
            "After Purge: Size of the Live Chain is: {} Blocks",
            live_chain_length(cfg.chain)
        );

        notify_follower(
            cfg.chain,
            &update_sender,
            &chain_update::Kind::ImmutableBlockRollForward,
        );
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

    // Live Fill data starts at fork 1.
    // Immutable data from a mithril snapshot is fork 0.
    // Live backfill is always Fork 1.
    let mut fork_count: u64 = 2;

    loop {
        // We never have a connection if we end up around the loop, so make a new one.
        let mut peer = persistent_reconnect(&cfg.relay_address, cfg.chain).await;

        match resync_live_tip(&mut peer, cfg.chain).await {
            Ok(tip) => debug!("Tip Resynchronized to {tip}"),
            Err(error) => {
                error!(
                    "Cardano Client {} failed to resync Tip: {}",
                    cfg.relay_address, error
                );
                continue;
            },
        }

        // Note: This can ONLY return with an error, otherwise it will sync indefinitely.
        if let Err(error) = follow_chain(&mut peer, cfg.chain, &mut fork_count).await {
            error!(
                "Cardano Client {} failed to follow chain: {}: Reconnecting.",
                cfg.relay_address, error
            );
            continue;
        }

        // If this returns, we are on a new fork (or assume we are)
        fork_count += 1;
    }
}

/// Is the current point aligned with what we know as tip.
pub(crate) async fn point_at_tip(chain: Network, point: &Point) -> bool {
    let tip = get_peer_tip(chain);

    // We are said to be AT TIP, if the block point is greater than or equal to the tip.
    tip <= *point
}
