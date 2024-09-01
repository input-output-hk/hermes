//! Cardano chain follow module.

use pallas::network::miniprotocols::txmonitor::{TxBody, TxId};
use tokio::sync::broadcast::{self};
use tracing::{debug, error};

use crate::{
    chain_sync::point_at_tip,
    chain_sync_live_chains::{find_best_fork_block, get_live_block, live_chain_length},
    chain_sync_ready::{block_until_sync_ready, get_chain_update_rx_queue},
    chain_update::{self, ChainUpdate},
    mithril_snapshot::MithrilSnapshot,
    mithril_snapshot_data::latest_mithril_snapshot_id,
    mithril_snapshot_iterator::MithrilSnapshotIterator,
    network::Network,
    point::{TIP_POINT, UNKNOWN_POINT},
    stats::{self, rollback},
    MultiEraBlock, Point, Statistics,
};

/// The Chain Follower
pub struct ChainFollower {
    /// The Blockchain network we are following.
    chain: Network,
    /// Where we end following.
    end: Point,
    /// Block we processed most recently.
    previous: Point,
    /// Where we are currently in the following process.
    current: Point,
    /// What fork were we last on
    fork: u64,
    /// Mithril Snapshot
    snapshot: MithrilSnapshot,
    /// Mithril Snapshot Follower
    mithril_follower: Option<MithrilSnapshotIterator>,
    /// Mithril TIP Reached
    mithril_tip: Option<Point>,
    /// Live Block Updates
    sync_updates: broadcast::Receiver<chain_update::Kind>,
}

impl ChainFollower {
    /// Follow a blockchain.
    ///
    /// # Arguments
    ///
    /// * `chain` - The blockchain network to follow.
    /// * `start` - The point or tip to start following from (inclusive).
    /// * `end` - The point or tip to stop following from (inclusive).
    ///
    /// # Returns
    ///
    /// The Chain Follower that will return blocks in the requested range.
    ///
    /// # Notes
    ///
    /// IF end < start, the follower will immediately yield no blocks.
    /// IF end is TIP, then the follower will continue to follow even when TIP is reached.
    /// Otherwise only blocks in the request range will be returned.
    ///
    /// Also, UNLIKE the blockchain itself, the only relevant information is the Slot#.
    /// The Block hash is not considered.
    /// If start is not an exact Slot#, then the NEXT Slot immediately following will be
    /// the first block returned.
    /// If the end is also not an exact Slot# with a block, then the last block will be
    /// the one immediately proceeding it.
    ///
    /// To ONLY follow from TIP, set BOTH start and end to TIP.
    #[must_use]
    pub async fn new(chain: Network, start: Point, end: Point) -> Self {
        let rx = get_chain_update_rx_queue(chain).await;

        ChainFollower {
            chain,
            end,
            previous: UNKNOWN_POINT,
            current: start,
            fork: 1, // This is correct, because Mithril is Fork 0.
            snapshot: MithrilSnapshot::new(chain),
            mithril_follower: None,
            mithril_tip: None,
            sync_updates: rx,
        }
    }

    /// If we can, get the next update from the mithril snapshot.
    async fn next_from_mithril(&mut self) -> Option<ChainUpdate> {
        let current_mithril_tip = latest_mithril_snapshot_id(self.chain).tip();

        if current_mithril_tip > self.current {
            if self.mithril_follower.is_none() {
                self.mithril_follower = self
                    .snapshot
                    .try_read_blocks_from_point(&self.current)
                    .await;
            }

            if let Some(follower) = self.mithril_follower.as_mut() {
                if let Some(next) = follower.next().await {
                    // debug!("Pre Previous update 3 : {:?}", self.previous);
                    self.previous = self.current.clone();
                    // debug!("Post Previous update 3 : {:?}", self.previous);
                    self.current = next.point();
                    self.fork = 0; // Mithril Immutable data is always Fork 0.
                    let update = ChainUpdate::new(chain_update::Kind::Block, false, next);
                    return Some(update);
                }
            }
        }

        if (self.mithril_tip.is_none() || current_mithril_tip > self.mithril_tip)
            && self.current < self.mithril_tip
        {
            let snapshot = MithrilSnapshot::new(self.chain);
            if let Some(block) = snapshot.read_block_at(&current_mithril_tip).await {
                // The Mithril Tip has moved forwards.
                self.mithril_tip = Some(current_mithril_tip);
                // Get the mithril tip block.
                let update =
                    ChainUpdate::new(chain_update::Kind::ImmutableBlockRollForward, false, block);
                return Some(update);
            }
            error!(
                tip = ?self.mithril_tip,
                current = ?current_mithril_tip,
                "Mithril Tip Block is not in snapshot. Should not happen."
            );
        }

        None
    }

    /// If we can, get the next update from the mithril snapshot.
    async fn next_from_live_chain(&mut self) -> Option<ChainUpdate> {
        let mut next_block: Option<MultiEraBlock> = None;
        let mut update_type = chain_update::Kind::Block;
        let mut rollback_depth: u64 = 0;

        // Special Case: point = TIP_POINT.  Just return the latest block in the live chain.
        if self.current == TIP_POINT {
            next_block = {
                let block = get_live_block(self.chain, &self.current, -1, false)?;
                Some(block)
            };
        }

        // In most cases we will be able to get the next block.
        if next_block.is_none() {
            // If we don't know the previous block, get the block requested.
            let advance = i64::from(!self.previous.is_unknown());
            next_block = get_live_block(self.chain, &self.current, advance, true);
        }

        // If we can't get the next consecutive block, then
        // Get the best previous block.
        if next_block.is_none() {
            debug!("No blocks left in live chain.");

            // IF this is an update still, and not us having caught up, then it WILL be a rollback.
            update_type = chain_update::Kind::Rollback;
            next_block = if let Some((block, depth)) =
                find_best_fork_block(self.chain, &self.current, &self.previous, self.fork)
            {
                debug!("Found fork block: {block}");
                // IF the block is the same as our current previous, there has been no chain
                // advancement, so just return None.
                if block.point().strict_eq(&self.current) {
                    None
                } else {
                    rollback_depth = depth;
                    Some(block)
                }
            } else {
                debug!("No block to find, rewinding to latest mithril tip.");
                let latest_mithril_point = latest_mithril_snapshot_id(self.chain).tip();
                if let Some(block) = MithrilSnapshot::new(self.chain)
                    .read_block_at(&latest_mithril_point)
                    .await
                {
                    rollback_depth = live_chain_length(self.chain) as u64;
                    Some(block)
                } else {
                    return None;
                }
            }
        }

        if let Some(next_block) = next_block {
            // Update rollback stats for the follower if one is reported.
            if update_type == chain_update::Kind::Rollback {
                rollback(self.chain, stats::RollbackType::Follower, rollback_depth);
            }
            // debug!("Pre Previous update 4 : {:?}", self.previous);
            self.previous = self.current.clone();
            // debug!("Post Previous update 4 : {:?}", self.previous);
            self.current = next_block.point().clone();
            self.fork = next_block.fork();

            let tip = point_at_tip(self.chain, &self.current).await;
            let update = ChainUpdate::new(update_type, tip, next_block);
            return Some(update);
        }

        None
    }

    /// Update the current Point, and return `false` if this fails.
    fn update_current(&mut self, update: &Option<ChainUpdate>) -> bool {
        if let Some(update) = update {
            let decoded = update.block_data().decode();
            self.current = Point::new(decoded.slot(), decoded.hash().to_vec());
            return true;
        }
        false
    }

    /// This is an unprotected version of `next()` which can ONLY be used within this
    /// crate. Its purpose is to allow the chain data to be inspected/validate prior
    /// to unlocking it for general access.
    ///
    /// This function can NOT return None, but that state is used to help process data.
    ///
    /// This function must not be exposed for general use.
    #[allow(clippy::unused_async)]
    pub(crate) async fn unprotected_next(&mut self) -> Option<ChainUpdate> {
        let mut update;

        // We will loop here until we can successfully return a new block
        loop {
            // Check if Immutable TIP has advanced, and if so, send a ChainUpdate about it.
            // Should only happen once every ~6hrs.
            // TODO.

            // Try and get the next update from the mithril chain, and return it if we are
            // successful.
            update = self.next_from_mithril().await;
            if update.is_some() {
                break;
            }

            // No update from Mithril Data, so try and get one from the live chain.
            update = self.next_from_live_chain().await;
            if update.is_some() {
                break;
            }

            // IF we can't get a new block directly from the mithril data, or the live chain, then
            // wait for something to change which might mean we can get the next block.
            let update = self.sync_updates.recv().await;
            match update {
                Ok(kind) => {
                    debug!("Update kind: {kind}");
                },
                Err(tokio::sync::broadcast::error::RecvError::Lagged(distance)) => {
                    debug!("Lagged by {} updates", distance);
                },
                Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                    // We are closed, so we need to wait for the next update.
                    // This is not an error.
                    return None;
                },
            }
        }

        // Update the current block, so we know which one to get next.
        if !self.update_current(&update) {
            return None;
        }

        update
    }

    /// Get the next block from the follower.
    /// Returns NONE is there is no block left to return.
    pub async fn next(&mut self) -> Option<ChainUpdate> {
        // If we aren't syncing TIP, and Current >= End, then return None
        if self.end != TIP_POINT && self.current >= self.end {
            return None;
        }

        // Can't follow if SYNC is not ready.
        block_until_sync_ready(self.chain).await;

        // Get next block from the iteration.
        self.unprotected_next().await
    }

    /// Get a single block from the chain by its point.
    ///
    /// If the Point does not point exactly at a block, it will return the next
    /// consecutive block.
    ///
    /// This is a convenience function which just used `ChainFollower` to fetch a single
    /// block.
    pub async fn get_block(chain: Network, point: Point) -> Option<ChainUpdate> {
        // Get the block from the chain.
        // This function suppose to run only once, so the end point
        // can be set to `TIP_POINT`
        let mut follower = Self::new(chain, point, TIP_POINT).await;
        follower.next().await
    }

    /// Get the current Immutable and live tips.
    ///
    /// Note, this will block until the chain is synced, ready to be followed.
    pub async fn get_tips(chain: Network) -> (Point, Point) {
        // Can't follow if SYNC is not ready.
        block_until_sync_ready(chain).await;

        let tips = Statistics::tips(chain);

        let mithril_tip = Point::fuzzy(tips.0);
        let live_tip = Point::fuzzy(tips.1);

        (mithril_tip, live_tip)
    }

    /// Schedule a transaction to be posted to the blockchain.
    ///
    /// # Arguments
    ///
    /// * `chain` - The blockchain to post the transaction on.
    /// * `txn` - The transaction to be posted.
    ///
    /// # Returns
    ///
    /// `TxId` - The ID of the transaction that was queued.
    #[allow(clippy::unused_async)]
    pub async fn post_txn(chain: Network, txn: TxBody) -> TxId {
        #[allow(clippy::no_effect_underscore_binding)]
        let _unused = chain;
        #[allow(clippy::no_effect_underscore_binding)]
        let _unused = txn;

        "unimplemented".to_string()
    }

    /// Check if a transaction, known by its `TxId`, has been sent to the Peer Node.
    ///
    /// Note, the `TxId` can ONLY be checked for ~6 hrs after it was posted.
    /// After which, it should be on the blockchain, and its the applications job to track
    /// if a transaction made it on-chain or not.
    #[allow(clippy::unused_async)]
    pub async fn txn_sent(chain: Network, id: TxId) -> bool {
        #[allow(clippy::no_effect_underscore_binding)]
        let _unused = chain;
        #[allow(clippy::no_effect_underscore_binding)]
        let _unused = id;

        false
    }
}

// TODO(SJ) - Add a function to check if a transaction is pending, or has been sent to the
// chain.

#[cfg(test)]
mod tests {
    use super::*;

    fn mock_block() -> MultiEraBlock {
        let raw_block = hex::decode(include_str!("./../test_data/shelley.block"))
            .expect("Failed to decode hex block.");

        let pallas_block = pallas::ledger::traverse::MultiEraBlock::decode(raw_block.as_slice())
            .expect("cannot decode block");

        let previous_point = Point::new(
            pallas_block.slot() - 1,
            pallas_block
                .header()
                .previous_hash()
                .expect("cannot get previous hash")
                .to_vec(),
        );

        MultiEraBlock::new(Network::Preprod, raw_block.clone(), &previous_point, 1)
            .expect("cannot create block")
    }

    #[tokio::test]
    async fn test_chain_follower_new() {
        let chain = Network::Mainnet;
        let start = Point::new(100u64, vec![]);
        let end = Point::fuzzy(999u64);

        let follower = ChainFollower::new(chain, start.clone(), end.clone()).await;

        assert_eq!(follower.chain, chain);
        assert_eq!(follower.end, end);
        assert_eq!(follower.previous, UNKNOWN_POINT);
        assert_eq!(follower.current, start);
        assert_eq!(follower.fork, 1);
        assert!(follower.mithril_follower.is_none());
        assert!(follower.mithril_tip.is_none());
    }

    #[tokio::test]
    async fn test_chain_follower_update_current_none() {
        let chain = Network::Mainnet;
        let start = Point::new(100u64, vec![]);
        let end = Point::fuzzy(999u64);

        let mut follower = ChainFollower::new(chain, start.clone(), end.clone()).await;

        let result = follower.update_current(&None);

        assert!(!result);
    }

    #[tokio::test]
    async fn test_chain_follower_update_current() {
        let chain = Network::Mainnet;
        let start = Point::new(100u64, vec![]);
        let end = Point::fuzzy(999u64);

        let mut follower = ChainFollower::new(chain, start.clone(), end.clone()).await;

        let block_data = mock_block();
        let update = ChainUpdate::new(chain_update::Kind::Block, false, block_data);

        let result = follower.update_current(&Some(update.clone()));

        assert!(result);
        assert_eq!(follower.current, update.block_data().point());
    }
}
