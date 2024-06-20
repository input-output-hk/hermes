//! Sync from the chain to an in-memory buffer.
//!
//! All iteration of the chain is done through this buffer or a mithril snapshot.
//! Consumers of this library do not talk to the node directly.

use std::time::Duration;

use crate::{
    error::{Error, Result},
    live_block::LiveBlock,
    ChainSyncConfig, MultiEraBlockData, Network, PointOrTip,
};

use anyhow::{bail, Context};
use crossbeam_skiplist::{SkipMap, SkipSet};
use once_cell::sync::Lazy;
use pallas::{
    ledger::traverse::MultiEraHeader,
    network::{
        facades::PeerClient,
        miniprotocols::{
            chainsync::{self, ClientError},
            Point,
        },
    },
};

use strum::IntoEnumIterator;
use tokio::{
    spawn,
    sync::mpsc,
    time::{sleep, timeout},
};
use tracing::{debug, error};

/// Type we use to manage the Sync Task handle map.
type LiveChainBlockList = SkipSet<LiveBlock>;
/// Handle to the mithril sync thread. One for each Network ONLY.
static LIVE_CHAINS: Lazy<SkipMap<Network, LiveChainBlockList>> = Lazy::new(|| {
    let map = SkipMap::new();
    for network in Network::iter() {
        map.insert(network, SkipSet::new());
    }
    map
});

/// The maximum number of seconds we wait for a node to connect.
const MAX_NODE_CONNECT_TIME_SECS: u64 = 2;

/// The maximum number of times we wait for a node to connect.
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

/// Set the Client Read Pointer for this connection with the Node
async fn set_client_read_pointer(client: &mut PeerClient, at: PointOrTip) -> Result<Point> {
    match at {
        PointOrTip::Point(Point::Origin) => client
            .chainsync()
            .intersect_origin()
            .await
            .map_err(Error::Chainsync),
        PointOrTip::Tip => client
            .chainsync()
            .intersect_tip()
            .await
            .map_err(Error::Chainsync),
        PointOrTip::Point(p @ Point::Specific(..)) => {
            match client.chainsync().find_intersect(vec![p]).await {
                Ok((point, _)) => match point {
                    Some(point) => Ok(point),
                    None => Err(Error::Chainsync(ClientError::IntersectionNotFound)),
                },
                Err(error) => Err(Error::Chainsync(error)),
            }
        },
    }
}

/// Resynchronize to the live tip in memory.
async fn resync_live_tip(
    client: &mut PeerClient, live_chain: &LiveChainBlockList,
) -> Result<Point> {
    let tip = match live_chain.back() {
        Some(live_block) => {
            let latest_block = live_block.value();
            PointOrTip::Point(latest_block.point.clone())
        },
        None => PointOrTip::Tip,
    };

    set_client_read_pointer(client, tip).await
}

/// Follows the chain until there is an error.
/// If this returns it can be assumed the client is disconnected.
///
/// We take ownership of the client because of that.
async fn follow_chain(
    peer: &mut PeerClient, live_chain: &LiveChainBlockList,
) -> anyhow::Result<()> {
    loop {
        debug!("Waiting for data from Cardano Peer Node:");

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
                // We can find if we are AT tip by comparing the current block Point with the tip Point.
                // We can estimate how far behind we are (in blocks) by subtracting current block
                // height and the tip block height.
                let decoded_header = MultiEraHeader::decode(
                    header.variant,
                    header.byron_prefix.map(|p| p.0),
                    &header.cbor,
                )
                .with_context(|| "Decoding Block Header")?;

                let point = Point::Specific(decoded_header.slot(), decoded_header.hash().to_vec());

                debug!("RollForward: {:?} {:?}", point, tip);

                // See if this block is the current nodes TIP.
                // let _at_tip = point == tip.0;

                let block_data = peer
                    .blockfetch()
                    .fetch_single(point.clone())
                    .await
                    .with_context(|| "Fetching block data")?;

                let live_block_data = MultiEraBlockData::new(block_data);

                // Add the live block to the head of the live chain
                live_chain.insert(LiveBlock::new(point, live_block_data));

                // TODO: Tell a follower the tip updated.
                //let update = if at_tip {
                // Then we are at the tip of the blockchain.
                //    ChainUpdate::BlockTip(MultiEraBlockData::new(block_data))
                //} else {
                //    ChainUpdate::Block(MultiEraBlockData::new(block_data))
                //};
            },
            chainsync::NextResponse::RollBackward(point, tip) => {
                debug!("RollBackward: {:?} {:?}", point, tip);

                // Purge the live data after this block.
                while let Some(tip_entry) = live_chain.back() {
                    let tip_block = tip_entry.value();
                    // If we got back to the rollback position then stop purging.
                    // Next update should be after this block, and arrive automatically.
                    if tip_entry.point == point {
                        break;
                    }
                    live_chain.remove(tip_block);
                }

                // TODO: Tell a follower we rolled back
                //Ok(Some(ChainUpdate::Rollback(MultiEraBlockData::new(
                //    block_data,
                //))))
            },
            chainsync::NextResponse::Await => {
                debug!("Peer Node says: Await");
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

/// Get the fill tp point for a chain.
async fn get_fill_to_point(live_chain: &SkipSet<LiveBlock>) -> Point {
    loop {
        match live_chain.front() {
            Some(entry) => return entry.value().point.clone(),
            None => {
                // Nothing in the Live chain to sync to, so wait until there is.
                tokio::time::sleep(Duration::from_secs(10)).await;
            },
        }
    }
}

/// Backfill the live chain, based on the Mithril Sync updates.
/// This does NOT return until the live chain has been backfilled from the end of mithril to the
/// current synced tip blocks.
///
/// This only needs to be done once per chain connection.
async fn live_sync_backfill(cfg: &ChainSyncConfig, from: Point) -> anyhow::Result<()> {
    // Get a reference to our live chain storage.
    // This SHOULD always exist, because its constructed by Lazy.
    let Some(live_block_list_entry) = LIVE_CHAINS.get(&cfg.chain) else {
        error!(
            "Internal Error: Chain Sync for: {} from  {} : Failed to find chain in LIVE_CHAINS",
            cfg.chain, cfg.relay_address,
        );
        bail!("Internal Error getting live chain.");
    };
    let live_chain = live_block_list_entry.value();

    let fill_to = get_fill_to_point(live_chain).await;
    let range = (from, fill_to);

    let range_msg = format!("{range:?}");

    let mut peer = persistent_reconnect(&cfg.relay_address, cfg.chain).await;

    // Request the range of blocks from the Peer.
    peer.blockfetch()
        .request_range(range)
        .await
        .with_context(|| "Requesting Block Range")?;

    while let Some(block_data) = peer.blockfetch().recv_while_streaming().await? {
        let block = MultiEraBlockData::new(block_data);
        let decoded_block = block.decode().with_context(|| "Decoding Block")?;
        let slot = decoded_block.slot();
        let hash = decoded_block.hash();
        let live_block = LiveBlock::new(Point::new(slot, hash.to_vec()), block);
        live_chain.insert(live_block);
        debug!("Backfilled Block: {}", slot);
    }

    debug!("Backfilled Range OK: {}", range_msg);

    Ok(())
}

/// Backfill and Purge the live chain, based on the Mithril Sync updates.
async fn live_sync_backfill_and_purge(cfg: ChainSyncConfig, mut rx: mpsc::Receiver<Point>) {
    let mut backfill: bool = true;

    // Get a reference to our live chain storage.
    // This SHOULD always exist, because its constructed by Lazy.
    let Some(live_block_list_entry) = LIVE_CHAINS.get(&cfg.chain) else {
        error!(
            "Internal Error: Chain Sync for: {} from  {} : Failed to find chain in LIVE_CHAINS",
            cfg.chain, cfg.relay_address,
        );
        return;
    };
    let live_chain = live_block_list_entry.value();

    loop {
        debug!("Size of the Live Chain is: {} Blocks", live_chain.len());

        let Some(point) = rx.recv().await else {
            error!("Mithril Sync Failed, can not continue chain sync either.");
            break;
        };

        if backfill {
            backfill = false;
            debug!("Mithril Tip has advanced to: {point:?} : BACKFILL");
            while let Err(error) = live_sync_backfill(&cfg, point.clone()).await {
                error!("Mithril Backfill Sync Failed: {}", error);
                sleep(Duration::from_secs(10)).await;
            }
        } else {
            debug!("Mithril Tip has advanced to: {point:?} : PURGE NEEDED");

            // Purge the live data before this block.
            while let Some(tip_entry) = live_chain.front() {
                let oldest_block = tip_entry.value();
                // If we got back to the rollback position then stop purging.
                // Next update should be after this block, and arrive automatically.
                if oldest_block.point == point {
                    break;
                }
                live_chain.remove(oldest_block);
            }
        }
    }

    // TODO: If the mithril sync dies, sleep for a bit and make sure the live chain doesn't grow
    // indefinitely.
    // We COULD move the spawn of mithril following into here, and if the rx dies, kill that task,
    // and restart it.
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
pub(crate) async fn chain_sync(cfg: ChainSyncConfig, rx: mpsc::Receiver<Point>) {
    debug!(
        "Chain Sync for: {} from {} : Starting",
        cfg.chain, cfg.relay_address,
    );

    // Get a reference to our live chain storage.
    // This SHOULD always exist, because its constructed by Lazy.
    let Some(live_block_list_entry) = LIVE_CHAINS.get(&cfg.chain) else {
        error!(
            "Internal Error: Chain Sync for: {} from  {} : Failed to find chain in LIVE_CHAINS",
            cfg.chain, cfg.relay_address,
        );
        return;
    };
    let live_chain = live_block_list_entry.value();

    let backfill_cfg = cfg.clone();

    // Start the Live chain backfill task.
    let _backfill_join_handle =
        spawn(async move { live_sync_backfill_and_purge(backfill_cfg.clone(), rx).await });

    loop {
        // We never have a connection if we end up around the loop, so make a new one.
        let mut peer = persistent_reconnect(&cfg.relay_address, cfg.chain).await;

        if let Err(error) = resync_live_tip(&mut peer, live_chain).await {
            // If we fail to resync the tip, then we should stop trying to sync.
            // We'll try again next time.
            error!(
                "Cardano Client {} failed to resync Tip: {}",
                cfg.relay_address, error
            );
            continue;
        };

        // Note: This can ONLY return with an error, otherwise it will sync indefinitely.
        if let Err(error) = follow_chain(&mut peer, live_chain).await {
            error!(
                "Cardano Client {} failed to follow chain: {}: Reconnecting.",
                cfg.relay_address, error
            );
            continue;
        }
    }
}
