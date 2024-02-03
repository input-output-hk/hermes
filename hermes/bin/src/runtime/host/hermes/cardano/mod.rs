//! Host - Cardano Blockchain implementations
//!
#![allow(unused_variables)]

use crate::runtime::extensions::hermes::cardano::api::{
    CardanoBlock, CardanoBlockchainId, CardanoTxn, FetchError, Host, Slot, TxnError,
    UnsubscribeOptions,
};

/// State
struct State {}

impl Host for State {
    #[doc = " Subscribe to the Blockchain block data."]
    #[doc = " "]
    #[doc = " **Parameters**"]
    #[doc = " "]
    #[doc = " - `net` : The blockchain network to fetch block from, and subscribe to."]
    #[doc = " - `whence`: Where to start fetching blocks from."]
    #[doc = " "]
    #[doc = " **Returns**"]
    #[doc = " "]
    #[doc = " - `ok(u64)` : The slot we are synching from now."]
    #[doc = " - `error(fetch-error)` : If an error occured."]
    #[doc = " "]
    #[doc = " **Notes**"]
    #[doc = " "]
    #[doc = " If the blockchain is not yet syncing, it will start, from the requested slot."]
    #[doc = " If the blockchain is not yet syncing, and `whence` == `continue` then the blockchain will"]
    #[doc = " not be synced from, the calling module will only be subscribed for block events."]
    #[doc = " "]
    #[doc = " If the blockchain is already syncing, the sync will stop and restart, unless `whence` == `continue`."]
    #[doc = " When `whence` == `continue` the blockchain will keep syncing from where it is at, and this module"]
    #[doc = " will be subscribed to block updates."]
    #[doc = " "]
    #[doc = " `whence` == `stop` will prevent the blockchain syncing, and the caller will be unsubscribed."]
    fn subscribe_blocks(
        &mut self, net: CardanoBlockchainId, whence: Slot,
    ) -> wasmtime::Result<Result<u64, FetchError>> {
        todo!()
    }

    #[doc = " Unsubscribe from the blockchain events listed."]
    #[doc = " "]
    #[doc = " **Parameters**"]
    #[doc = " "]
    #[doc = " - `events` : The events to unsubscribe from (and optionally stop the blockchain follower)."]
    #[doc = " "]
    #[doc = " **Notes**"]
    #[doc = " "]
    #[doc = " This only unsubscribes from the events."]
    #[doc = " The option `stop` MUST be set to actually stop fetching data from the blockchain once started."]
    #[doc = " "]
    #[doc = " `stop` can be set without unsubscribing, and this will interrupt the flow of blockchain data."]
    #[doc = " After `stop`,  `subscribe-blocks(?, continue)` would cause blockchain sync to continue from"]
    #[doc = " the last block received.  This would result in the last block being sent as an event twice,"]
    #[doc = " once before the `stop` and once after the `continue`."]
    fn unsubscribe(&mut self, events: UnsubscribeOptions) -> wasmtime::Result<()> {
        todo!()
    }

    #[doc = " Subscribe to transaction data events, does not alter the blockchain sync in anyway."]
    #[doc = " "]
    #[doc = " **Parameters**"]
    #[doc = " "]
    #[doc = " - `net` : The blockchain network to subscribe to txn events from."]
    fn subscribe_txn(&mut self, net: CardanoBlockchainId) -> wasmtime::Result<()> {
        todo!()
    }

    #[doc = " Subscribe to blockchain rollback events, does not alter the blockchain sync in anyway."]
    #[doc = " "]
    #[doc = " **Parameters**"]
    #[doc = " "]
    #[doc = " - `net` : The blockchain network to subscribe to txn events from."]
    #[doc = " "]
    #[doc = " **Notes**"]
    #[doc = " "]
    #[doc = " After a rollback event, the blockchain sync will AUTOMATICALLY start sending block"]
    #[doc = " data from the rollback point.  No action is required to actually follow the rollback, unless the"]
    #[doc = " default behavior is not desired."]
    fn subscribe_rollback(&mut self, net: CardanoBlockchainId) -> wasmtime::Result<()> {
        todo!()
    }

    #[doc = " Fetch a block from the requested blockchain at the requested slot."]
    #[doc = " "]
    #[doc = " **Parameters**"]
    #[doc = " "]
    #[doc = " - `net`    : The blockchain network to get a block from."]
    #[doc = " - `whence` : Which block to get."]
    #[doc = " "]
    #[doc = " **Returns**"]
    #[doc = " "]
    #[doc = " - `cardano-block` : The block requested."]
    #[doc = " - `fetch-error` : An error if the block can not be fetched."]
    #[doc = " "]
    #[doc = " **Notes**"]
    #[doc = " "]
    #[doc = " Fetching a block does not require the blockchain to be subscribed, or for blocks to be"]
    #[doc = " being followed and generating events."]
    #[doc = " It also will not alter the automatic fetching of blocks in any way, and happens in parallel"]
    #[doc = " to automated block fetch."]
    fn fetch_block(
        &mut self, net: CardanoBlockchainId, whence: Slot,
    ) -> wasmtime::Result<Result<CardanoBlock, FetchError>> {
        todo!()
    }

    #[doc = " Get transactions from a block."]
    #[doc = " "]
    #[doc = " This can be used to easily extract all transactions from a complete block."]
    #[doc = " "]
    #[doc = " **Parameters**"]
    #[doc = " "]
    #[doc = " - `block` : The blockchain data to extract transactions from."]
    #[doc = " "]
    #[doc = " **Returns**"]
    #[doc = " "]
    #[doc = " - a list of all transactions in the block, in the order they appear in the block."]
    #[doc = " "]
    #[doc = " **Notes**"]
    #[doc = " "]
    #[doc = " This function exists to support `fetch-block`."]
    #[doc = " Transactions from subscribed block events, should be processed as transaction events."]
    fn get_txns(&mut self, block: CardanoBlock) -> wasmtime::Result<Vec<CardanoTxn>> {
        todo!()
    }

    #[doc = " Post a transactions to the blockchain."]
    #[doc = " "]
    #[doc = " This can be used to post a pre-formed transaction to the required blockchain."]
    #[doc = " "]
    #[doc = " **Parameters**"]
    #[doc = " "]
    #[doc = " - `net` : The blockchain to post the transaction to."]
    #[doc = " - `txn` : The transaction data, ready to submit."]
    #[doc = " "]
    #[doc = " **Returns**"]
    #[doc = " "]
    #[doc = " - An error if the transaction can not be posted."]
    #[doc = " "]
    #[doc = " **Notes**"]
    #[doc = " "]
    #[doc = " This is proposed functionality, and is not yet active."]
    #[doc = " All calls to this function will return `post-txn-not-allowed` error."]
    fn post_txn(
        &mut self, net: CardanoBlockchainId, txn: CardanoTxn,
    ) -> wasmtime::Result<Result<(), TxnError>> {
        todo!()
    }
}
