//! Cardano chain follow module.

use pallas::network::miniprotocols::txmonitor::{TxBody, TxId};
use tokio::sync::broadcast;
use tracing::{debug, error};

use crate::{
    chain_sync_live_chains::get_live_block_after,
    chain_sync_ready::{block_until_sync_ready, get_chain_update_rx_queue},
    chain_update::{self, ChainUpdate},
    mithril_snapshot::MithrilSnapshot,
    mithril_snapshot_iterator::MithrilSnapshotIterator,
    network::Network,
    point::TIP_POINT,
    Point,
};

/// The Chain Follower
pub struct ChainFollower {
    /// The Blockchain network we are following.
    chain: Network,
    /// Where we end following.
    end: Point,
    /// Where we are currently in the following process.
    current: Point,
    /// Mithril Snapshot
    snapshot: MithrilSnapshot,
    /// Mithril Snapshot Follower
    mithril_follower: Option<MithrilSnapshotIterator>,
    /// Live Block Updates
    sync_updates: broadcast::Receiver<ChainUpdate>,
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
            current: start,
            snapshot: MithrilSnapshot::new(chain),
            mithril_follower: None,
            sync_updates: rx,
        }
    }

    /// If we can, get the next update from the mithril snapshot.
    async fn next_from_mithril(&mut self, point: &Point) -> Option<ChainUpdate> {
        if self.mithril_follower.is_none() {
            self.mithril_follower = self.snapshot.try_read_blocks_from_point(point).await;
        }

        if let Some(follower) = self.mithril_follower.as_mut() {
            if let Some(next) = follower.next().await {
                let update = ChainUpdate::new(chain_update::Kind::Block, false, next);
                return Some(update);
            }
        }
        None
    }

    /// If we can, get the next update from the mithril snapshot.
    #[allow(clippy::unused_self)]
    fn next_from_live_chain(&mut self, point: &Point) -> Option<ChainUpdate> {
        get_live_block_after(self.chain, point)
            .map(|live_block| ChainUpdate::new(chain_update::Kind::Block, false, live_block))
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

    /// Get an update from the live chain status follower.
    async fn update_live_event(&mut self) -> Option<ChainUpdate> {
        loop {
            debug!(
                "Waiting for update from blockchain. Last Block: {:?}",
                self.current
            );
            let update = self.sync_updates.recv().await;

            match update {
                Ok(update) => {
                    if update.kind == chain_update::Kind::Block && update.immutable() {
                        // Shouldn't happen, log if it does, but otherwise ignore it.
                        error!(
                            chain = self.chain.to_string(),
                            "Received an ImmutableBlock update, these are not Live Updates.  Ignored."
                        );
                    } else {
                        // Due to the small buffering window, its possible we already processed a
                        // block in the live queue from the live in-memory
                        // chain. This is not an error, so just ignore that
                        // entry in the queue.
                        if update.kind != chain_update::Kind::Rollback
                            && self.current >= update.data.point()
                        {
                            debug!("Discarding: {}", update);
                            continue;
                        }
                        return Some(update);
                    }
                },
                Err(error) => {
                    match error {
                        broadcast::error::RecvError::Closed => {},
                        broadcast::error::RecvError::Lagged(_) => continue, /* Lagged just means
                                                                             * we need to read
                                                                             * again. */
                    }
                    error!(
                        chain = self.chain.to_string(),
                        "Live Chain follower error: {error}"
                    );
                    return None;
                },
            }
        }
    }

    /// This is an unprotected version of `next()` which can ONLY be used within this
    /// crate. Its purpose is to allow the chain data to be inspected/validate prior
    /// to unlocking it for general access.
    ///
    /// This function must not be exposed for general use.
    #[allow(clippy::unused_async)]
    pub(crate) async fn unprotected_next(&mut self) -> Option<ChainUpdate> {
        // If we are following TIP, then we just wait for Tip Updates.
        let update = if self.current == TIP_POINT {
            self.update_live_event().await
        } else if let Some(update) = self.next_from_mithril(&self.current.clone()).await {
            Some(update)
        } else if let Some(update) = self.next_from_live_chain(&self.current.clone()) {
            Some(update)
        } else {
            self.update_live_event().await
        };

        if !self.update_current(&update) {
            return None;
        }

        update
    }

    /// Get the next block from the follower.
    /// Returns NONE is there is no block left to return.
    pub async fn next(&mut self) -> Option<ChainUpdate> {
        // If we aren't syncing TIP, and Current >= End, then return None
        if self.end != TIP_POINT && self.current > self.end {
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
        let mut follower = Self::new(chain, point.clone(), point).await;

        follower.next().await
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
