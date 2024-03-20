//!  Cardano Blockchain host implementation for WASM runtime.

use crate::{
    runtime_context::HermesRuntimeContext,
    runtime_extensions::bindings::hermes::cardano::api::{
        CardanoBlock, CardanoBlockchainId, CardanoTxn, FetchError, Host, Slot, TxnError,
        UnsubscribeOptions,
    },
};

impl Host for HermesRuntimeContext {
    /// Subscribe to the Blockchain block data.
    ///
    /// **Parameters**
    ///
    /// - `net` : The blockchain network to fetch block from, and subscribe to.
    /// - `whence`: Where to start fetching blocks from.
    ///
    /// **Returns**
    ///
    /// - `ok(u64)` : The slot we are synching from now.
    /// - `error(fetch-error)` : If an error occurred.
    ///
    /// **Notes**
    ///
    /// If the blockchain is not yet syncing, it will start, from the requested slot.
    /// If the blockchain is not yet syncing, and `whence` == `continue` then the
    /// blockchain will
    /// not be synced from, the calling module will only be subscribed for block events.
    ///
    /// If the blockchain is already syncing, the sync will stop and restart, unless
    /// `whence` == `continue`.
    /// When `whence` == `continue` the blockchain will keep syncing from where it is at,
    /// and this module
    /// will be subscribed to block updates.
    ///
    /// `whence` == `stop` will prevent the blockchain syncing, and the caller will be
    /// unsubscribed.
    fn subscribe_blocks(
        &mut self, net: CardanoBlockchainId, whence: Slot,
    ) -> wasmtime::Result<Result<u64, FetchError>> {
        let sub_type = match whence {
            Slot::Genesis => {
                super::SubscriptionType::Blocks(cardano_chain_follower::PointOrTip::Point(
                    cardano_chain_follower::Point::Origin,
                ))
            },
            Slot::Point((slot, hash)) => {
                super::SubscriptionType::Blocks(cardano_chain_follower::PointOrTip::Point(
                    cardano_chain_follower::Point::Specific(slot, hash),
                ))
            },
            Slot::Tip => super::SubscriptionType::Blocks(cardano_chain_follower::PointOrTip::Tip),
            Slot::Continue => super::SubscriptionType::Continue,
        };

        let res = super::subscribe(
            net,
            self.app_name().clone(),
            self.module_id().clone(),
            sub_type,
        );

        match res {
            Ok(_) => Ok(Ok(0)),
            Err(_) => Ok(Err(FetchError::InvalidSlot)),
        }
    }

    /// Unsubscribe from the blockchain events listed.
    ///
    /// **Parameters**
    ///
    /// - `opts` : The events to unsubscribe from (and optionally stop the blockchain
    ///   follower).
    ///
    /// **Notes**
    ///
    /// This only unsubscribes from the events.
    /// The option `stop` MUST be set to actually stop fetching data from the blockchain
    /// once started.
    ///
    /// `stop` can be set without unsubscribing, and this will interrupt the flow of
    /// blockchain data.
    /// After `stop`,  `subscribe-blocks(?, continue)` would cause blockchain sync to
    /// continue from
    /// the last block received.  This would result in the last block being sent as an
    /// event twice,
    /// once before the `stop` and once after the `continue`.
    fn unsubscribe(
        &mut self, net: CardanoBlockchainId, opts: UnsubscribeOptions,
    ) -> wasmtime::Result<()> {
        super::unsubscribe(net, self.app_name().clone(), self.module_id().clone(), opts)
            .map_err(wasmtime::Error::new)
    }

    /// Subscribe to transaction data events, does not alter the blockchain sync in
    /// anyway.
    ///
    /// **Parameters**
    ///
    /// - `net` : The blockchain network to subscribe to txn events from.
    fn subscribe_txn(&mut self, net: CardanoBlockchainId) -> wasmtime::Result<()> {
        super::subscribe(
            net,
            self.app_name().clone(),
            self.module_id().clone(),
            super::SubscriptionType::Transactions,
        )
        .map_err(wasmtime::Error::new)?;

        Ok(())
    }

    /// Subscribe to blockchain rollback events, does not alter the blockchain sync in
    /// anyway.
    ///
    /// **Parameters**
    ///
    /// - `net` : The blockchain network to subscribe to txn events from.
    ///
    /// **Notes**
    ///
    /// After a rollback event, the blockchain sync will AUTOMATICALLY start sending block
    /// data from the rollback point.  No action is required to actually follow the
    /// rollback, unless the
    /// default behavior is not desired.
    fn subscribe_rollback(&mut self, net: CardanoBlockchainId) -> wasmtime::Result<()> {
        super::subscribe(
            net,
            self.app_name().clone(),
            self.module_id().clone(),
            super::SubscriptionType::Rollbacks,
        )
        .map_err(wasmtime::Error::new)?;

        Ok(())
    }

    /// Fetch a block from the requested blockchain at the requested slot.
    ///
    /// **Parameters**
    ///
    /// - `net`    : The blockchain network to get a block from.
    /// - `whence` : Which block to get.
    ///
    /// **Returns**
    ///
    /// - `cardano-block` : The block requested.
    /// - `fetch-error` : An error if the block can not be fetched.
    ///
    /// **Notes**
    ///
    /// Fetching a block does not require the blockchain to be subscribed, or for blocks
    /// to be
    /// being followed and generating events.
    /// It also will not alter the automatic fetching of blocks in any way, and happens in
    /// parallel
    /// to automated block fetch.
    fn fetch_block(
        &mut self, net: CardanoBlockchainId, whence: Slot,
    ) -> wasmtime::Result<Result<CardanoBlock, FetchError>> {
        let at = match whence {
            Slot::Genesis => {
                cardano_chain_follower::PointOrTip::Point(cardano_chain_follower::Point::Origin)
            },
            Slot::Point((slot, hash)) => {
                cardano_chain_follower::PointOrTip::Point(cardano_chain_follower::Point::Specific(
                    slot, hash,
                ))
            },
            Slot::Tip => cardano_chain_follower::PointOrTip::Tip,
            Slot::Continue => todo!(),
        };

        let block_data = super::read_block(net, at).unwrap();

        Ok(Ok(block_data.into_raw_data()))
    }

    /// Get transactions from a block.
    ///
    /// This can be used to easily extract all transactions from a complete block.
    ///
    /// **Parameters**
    ///
    /// - `block` : The blockchain data to extract transactions from.
    ///
    /// **Returns**
    ///
    /// - a list of all transactions in the block, in the order they appear in the block.
    ///
    /// **Notes**
    ///
    /// This function exists to support `fetch-block`.
    /// Transactions from subscribed block events, should be processed as transaction
    /// events.
    fn get_txns(&mut self, _block: CardanoBlock) -> wasmtime::Result<Vec<CardanoTxn>> {
        todo!()
    }

    /// Post a transactions to the blockchain.
    ///
    /// This can be used to post a pre-formed transaction to the required blockchain.
    ///
    /// **Parameters**
    ///
    /// - `net` : The blockchain to post the transaction to.
    /// - `txn` : The transaction data, ready to submit.
    ///
    /// **Returns**
    ///
    /// - An error if the transaction can not be posted.
    ///
    /// **Notes**
    ///
    /// This is proposed functionality, and is not yet active.
    /// All calls to this function will return `post-txn-not-allowed` error.
    fn post_txn(
        &mut self, _net: CardanoBlockchainId, _txn: CardanoTxn,
    ) -> wasmtime::Result<Result<(), TxnError>> {
        todo!()
    }
}
