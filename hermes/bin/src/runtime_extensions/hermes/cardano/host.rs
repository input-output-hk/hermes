//!  Cardano Blockchain host implementation for WASM runtime.

use cardano_chain_follower::PointOrTip;

use crate::{
    runtime_context::HermesRuntimeContext,
    runtime_extensions::bindings::hermes::cardano::api::{
        CardanoBlock, CardanoBlockchainId, CardanoTxn, FetchError, Host, Slot, SubscribeOptions,
        TxnError,
    },
};

/// Convert a `whence` parameter into a `Point`.
fn whence_to_point(whence: Slot) -> Option<PointOrTip> {
    match whence {
        Slot::Genesis => Some(cardano_chain_follower::Point::Origin.into()),
        Slot::Point((slot, hash)) => {
            Some(cardano_chain_follower::Point::Specific(slot, hash).into())
        },
        Slot::Tip => Some(cardano_chain_follower::PointOrTip::Tip),
        Slot::Continue => None,
    }
}

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
    fn subscribe(
        &mut self, net: CardanoBlockchainId, whence: Slot, what: SubscribeOptions,
    ) -> wasmtime::Result<Result<(), FetchError>> {
        // Convert whence to an actual Point or None if we don't have one at all.
        let whence = whence_to_point(whence);

        let res = super::subscribe(
            net,
            self.app_name().clone(),
            self.module_id().clone(),
            whence,
            what,
        );

        match res {
            Ok(()) => Ok(Ok(())),
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
        &mut self, net: CardanoBlockchainId, opts: SubscribeOptions,
    ) -> wasmtime::Result<()> {
        super::unsubscribe(net, self.app_name().clone(), self.module_id().clone(), opts)
        // .map_err(|e| wasmtime::Error::new(e))
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
        // Convert whence to an actual Point or None if we don't have one at all.
        let Some(at) = whence_to_point(whence) else {
            return Ok(Err(FetchError::InvalidSlot));
        };

        match super::read_block(net, at) {
            Ok(block_data) => Ok(Ok(block_data.into_raw_data())),
            Err(_) => Ok(Err(FetchError::InvalidSlot)),
        }
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
    fn get_txns(&mut self, block: CardanoBlock) -> wasmtime::Result<Vec<CardanoTxn>> {
        let block_data = pallas::ledger::traverse::MultiEraBlock::decode(&block)?;

        Ok(block_data.txs().into_iter().map(|tx| tx.encode()).collect())
    }

    /// Subscribe to transaction data events, does not alter the blockchain sync in
    /// anyway.
    ///
    /// **Parameters**
    ///
    /// - `net` : The blockchain network to subscribe to txn events from.
    fn fetch_txn(
        &mut self, net: CardanoBlockchainId, whence: Slot, offset: u16,
    ) -> wasmtime::Result<Result<CardanoTxn, FetchError>> {
        // Convert whence to an actual Point or None if we don't have one at all.
        let Some(at) = whence_to_point(whence) else {
            return Ok(Err(FetchError::InvalidSlot));
        };

        match super::read_block(net, at) {
            Ok(block_data) => {
                let Ok(decoded_block) = block_data.decode() else {
                    // Shouldn't happen, but if it does say the Slot is invalid.
                    return Ok(Err(FetchError::InvalidSlot));
                };
                let txs = decoded_block.txs();
                let Some(txn) = txs.get(offset as usize) else {
                    return Ok(Err(FetchError::InvalidTxn));
                };
                Ok(Ok(txn.encode()))
            },
            Err(_) => Ok(Err(FetchError::InvalidSlot)),
        }
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
