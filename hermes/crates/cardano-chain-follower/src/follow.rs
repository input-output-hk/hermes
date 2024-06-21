//! Cardano chain follow module.
use crate::{
    chain_sync::block_until_sync_ready, chain_update::ChainUpdate, network::Network,
    point_or_tip::PointOrTip,
};

/// The Chain Follower
pub struct ChainFollower {
    /// The Blockchain network we are following.
    chain: Network,
    /// Where we start following from.
    start: PointOrTip,
    /// Where we end following.
    end: PointOrTip,
    /// Where we are currently in the following process.
    current: PointOrTip,
}

impl ChainFollower {
    /// Follow a blockchain.
    ///
    /// # Arguments
    ///
    /// * `chain` - The blockchain network to follow.
    /// * `start` - The point or tip to start following from (exclusive).
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
    /// If start is not an exact Slot#, then the NEXT Slot immediately following will be the
    /// first block returned.
    /// If the end is also not an exact Slot# with a block, then the last block will be the one
    /// immediately proceeding it.
    ///
    /// To ONLY follow from TIP, set BOTH start and end to TIP.
    #[must_use]
    pub fn new(chain: Network, start: PointOrTip, end: PointOrTip) -> Self {
        ChainFollower {
            chain,
            start: start.clone(),
            end,
            current: start,
        }
    }

    /// Get the next block from the follower.
    /// Returns NONE is there is no block left to return.
    #[allow(clippy::unused_async)]
    pub async fn next(&mut self) -> Option<ChainUpdate> {
        // If we aren't syncing TIP, and Current >= End, then return None
        if self.end != PointOrTip::Tip && self.current >= self.end {
            return None;
        }

        // Can't follow if SYNC is not ready.
        block_until_sync_ready(self.chain).await;

        let _unused = self.start.clone();

        // Get the next block after Current

        // Set Current to the next block
        None
    }
}

// TODO(SJ) - Add function to get a single block from the chain.

// TODO(SJ) - Add a function to post a transaction to the chain.

// TODO(SJ) - Add a function to check if a transaction is pending, or has been sent to the chain.
