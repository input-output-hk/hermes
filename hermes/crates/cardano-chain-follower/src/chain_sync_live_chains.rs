//! Storage of each Live Chain per Blockchain.

use std::time::Duration;

use crossbeam_skiplist::{map::Entry, SkipMap, SkipSet};
use once_cell::sync::Lazy;
use pallas::network::miniprotocols::Point;
use strum::{Display, IntoEnumIterator};
use tracing::{debug, error};

use crate::{MultiEraBlock, Network, PointOrTip};

/// Type we use to manage the Sync Task handle map.
pub(crate) type LiveChainBlockList = SkipSet<MultiEraBlock>;
/// Handle to the mithril sync thread. One for each Network ONLY.
static LIVE_CHAINS: Lazy<SkipMap<Network, LiveChainBlockList>> = Lazy::new(|| {
    let map = SkipMap::new();
    for network in Network::iter() {
        map.insert(network, LiveChainBlockList::new());
    }
    map
});

/// Number of seconds to wait if we detect a `SyncReady` race condition.
const DATA_RACE_BACKOFF_SECS: u64 = 2;

/// Get the Live block immediately following the specified block.
pub(crate) fn get_live_block_after(chain: Network, point: &Point) -> Option<MultiEraBlock> {
    if let Some(live_chain_entry) = LIVE_CHAINS.get(&chain) {
        let live_chain = live_chain_entry.value();
        let probe_block = MultiEraBlock::probe(chain, point);
        let this_block = live_chain.get(&probe_block)?;
        let next_block = this_block.next()?;
        let next_block_value = next_block.value().clone();

        return Some(next_block_value);
    };
    None
}

/// Get the Live block at a particular point.
pub(crate) fn get_live_block_at(chain: Network, point: &Point) -> Option<MultiEraBlock> {
    if let Some(live_chain_entry) = LIVE_CHAINS.get(&chain) {
        let live_chain = live_chain_entry.value();
        let probe_block = MultiEraBlock::probe(chain, point);
        let this_block = live_chain.get(&probe_block)?;
        let this_block_value = this_block.value().clone();

        return Some(this_block_value);
    };
    None
}

/// Get the Live block before a particular point.
#[allow(dead_code)]
pub(crate) fn get_live_block_before(chain: Network, point: &Point) -> Option<MultiEraBlock> {
    if let Some(live_chain_entry) = LIVE_CHAINS.get(&chain) {
        let live_chain = live_chain_entry.value();
        let probe_block = MultiEraBlock::probe(chain, point);
        let this_block = live_chain.get(&probe_block)?;
        let previous_block = this_block.prev()?;
        let previous_block_value = previous_block.value().clone();

        return Some(previous_block_value);
    };
    None
}

/// Get the `LiveChainBlockList` for a particular `Network`.
fn get_live_chain(chain: Network) -> Entry<'static, Network, LiveChainBlockList> {
    // Get a reference to our live chain storage.
    // This SHOULD always exist, because its initialized exhaustively.
    // If this FAILS, Recreate a blank chain, but log an error as its a serious UNRECOVERABLE
    // BUG.
    if let Some(entry) = LIVE_CHAINS.get(&chain) {
        return entry;
    }

    error!(
        chain = chain.to_string(),
        "Internal Error: Chain Sync Failed to find chain in LIVE_CHAINS"
    );
    let new_chain = LiveChainBlockList::new();
    LIVE_CHAINS.insert(chain, new_chain);

    // This should NOT fail, because we just inserted it, its catastrophic failure if it does.
    #[allow(clippy::expect_used)]
    LIVE_CHAINS
        .get(&chain)
        .expect("Internal Error: Chain Sync Failed to find chain in LIVE_CHAINS")
}

/// Get the fill tp point for a chain.
pub(crate) async fn get_fill_to_point(chain: Network) -> Point {
    let live_chain_entry = get_live_chain(chain);
    let live_chain = live_chain_entry.value();

    loop {
        match live_chain.front() {
            Some(entry) => return entry.value().point(),
            None => {
                // Nothing in the Live chain to sync to, so wait until there is.
                tokio::time::sleep(Duration::from_secs(DATA_RACE_BACKOFF_SECS)).await;
            },
        }
    }
}

/// Insert a block into the live chain (in-order).
pub(crate) fn live_chain_insert(chain: Network, block: MultiEraBlock) {
    let live_chain_entry = get_live_chain(chain);
    let live_chain = live_chain_entry.value();

    live_chain.insert(block);
}

/// Get the length of the live chain.
/// Probably used by debug code only, so its ok if this is not use.
#[allow(dead_code)]
pub(crate) fn live_chain_length(chain: Network) -> usize {
    let live_chain_entry = get_live_chain(chain);
    let live_chain = live_chain_entry.value();
    live_chain.len()
}

/// The type of chain purge to perform.
#[derive(Copy, Clone, Display)]
pub(crate) enum PurgeType {
    /// Purge from the Oldest chronological blocks
    Oldest,
    /// Purge from the Newest chronological blocks
    Newest,
}

/// Purge the live chain up to a particular point.
/// This is used for both rollback processing and
pub(crate) fn purge_live_chain(chain: Network, point: &Point, purge_type: PurgeType) {
    let live_chain_entry = get_live_chain(chain);
    let live_chain = live_chain_entry.value();

    let mut purged_blocks: u64 = 0;

    debug!(
        "Purging Live Chain upto from {purge_type}: {point:?}. Size of the Chain: {}",
        live_chain.len()
    );

    // Purge the live data before this block.
    loop {
        let next_entry = match purge_type {
            PurgeType::Oldest => live_chain.front(),
            PurgeType::Newest => live_chain.back(),
        };

        if let Some(tip_entry) = next_entry {
            let next_block = tip_entry.value();
            // If we got to the purge position then stop purging.
            // Next update should be after this block, and arrive automatically.
            if next_block.point() == *point {
                break;
            }
            live_chain.remove(next_block);
            purged_blocks += 1;
        } else {
            break;
        }
    }

    debug!(
        "Purged {} Blocks. Size of the Live Chain is: {} Blocks",
        purged_blocks,
        live_chain.len()
    );
}

/// Get the latest point recorded in the live chain, or TIP if nothing is recorded.
pub(crate) fn latest_live_point(chain: Network) -> PointOrTip {
    let live_chain_entry = get_live_chain(chain);
    let live_chain = live_chain_entry.value();

    if let Some(live_block) = live_chain.back() {
        let latest_block = live_block.value();
        let latest_point = latest_block.point();

        return PointOrTip::Point(latest_point);
    }

    PointOrTip::Tip
}

/// If we fail to sync on our last known tip, we use this to purge it, and try again.
pub(crate) fn purge_latest_live_point(chain: Network) {
    let live_chain_entry = get_live_chain(chain);
    let live_chain = live_chain_entry.value();

    let _unused = live_chain.pop_back();
}
