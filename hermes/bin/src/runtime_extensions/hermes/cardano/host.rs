//!  Cardano Blockchain host implementation for WASM runtime.

use crate::{
    runtime_context::HermesRuntimeContext,
    runtime_extensions::bindings::hermes::cardano::api::{
        Block, CardanoNetwork, Cbor, CreateNetworkError, HostBlock, HostNetwork, HostTransaction,
        Network, Slot, SubscribeError, SubscriptionId, SyncSlot, Transaction, TxnHash, TxnIdx,
    },
};

impl HostNetwork for HermesRuntimeContext {
    /// Create a new Cardano network instance.
    //
    /// **Parameters**
    //
    /// - `network`: The Cardano network to connect to (e.g., Mainnet, Preprod, Preview).
    //
    /// **Returns**
    //
    /// - `ok(_)`: If successfully create network.
    /// - `error(create-network-error)`: If creating network failed.
    fn new(&mut self, network: CardanoNetwork) -> wasmtime::Result<Result<(), CreateNetworkError>> {
        todo!()
    }

    /// Subscribe to blockchain block events, start from a specified starting point.
    //
    /// This sets up a subscription to receive new block and block rollback updates starting from the
    /// given `start`.
    ///
    /// **Parameters**
    ///
    /// - `start`: The slot to begin following from.
    ///
    /// **Returns**
    ///
    /// - `ok(subscription-id)`: A unique identifier of this subscription.
    ///                         Use to distinguishes events from different subscribers
    ///                         and provides control over subscription management.
    ///                         The ID must be unique across all active subscriptions.
    /// - `error(subscribe-error)`: If subscription failed.
    fn subscribe_block(
        &mut self, self_: wasmtime::component::Resource<Network>, start: SyncSlot,
    ) -> wasmtime::Result<Result<SubscriptionId, SubscribeError>> {
        todo!()
    }

    /// Subscribe to blockchain immutable rolls forward.
    ///
    /// This sets up a subscription to receive event when the immutable part of the blockchain
    /// roll forwards.
    ///
    /// **Parameters**
    ///
    /// - `start`: The slot to begin following from.
    ///
    /// **Returns**
    ///
    /// - `ok(subscription-id)`: A unique identifier of this subscription.
    ///                         Use to distinguishes events from different subscribers
    ///                         and provides control over subscription management.
    ///                         The ID must be unique across all active subscriptions.
    /// - `error(subscribe-error)`: If subscription failed.
    fn subscribe_immutable_roll_forward(
        &mut self, self_: wasmtime::component::Resource<Network>, start: SyncSlot,
    ) -> wasmtime::Result<Result<SubscriptionId, SubscribeError>> {
        todo!()
    }

    /// Unsubscribing block events given an ID.
    /// Once this function is called, the subscription instance, `subscription-id` will be removed.
    ///
    /// **Parameters**
    /// - `id` : A unique identifier of the block subscription to unsubscribe from.
    ///         This `id` is returned from the `subscribe-block` or `subscribe-immutable-roll-forward`
    fn unsubscribe(
        &mut self, self_: wasmtime::component::Resource<Network>, id: SubscriptionId,
    ) -> wasmtime::Result<()> {
        todo!()
    }

    /// Get a block relative to `start` by `step`.
    ///
    ///  **Parameters**
    ///  - `start`: Slot to begin retrieval from, current tip if `None`.
    ///  - `step`
    ///      -`0` : the block at `start`, will return `None` if there is no block exactly at this `start` slot.
    ///      -`+n`: the `n`‑th block *after* the given `start` slot.
    ///      –`‑n`: the `n`‑th block *before* the given `start` slot.
    ///    
    ///  Note: For both `+n` and `-n`, the `start` does not need to be a true block.
    ///  They will return the block which appears at this block offset, given the arbitrary start point.
    ///  IF the `start` block does exist, it will never returned with a positive or negative `step`, as it is `step` 0.
    ///
    ///  Example, Given three consecutive blocks at slots `100`, `200` and `300` the following will be returned:
    ///      - `start = 100, step = 0` -> 100 (Exact match)
    ///      - `start = 100, step = 2` -> 300 (Skips 200)
    ///      - `start = 150, step = 1` -> 200 (Rounds up from 150)
    ///      - `start = 200, step = 1` -> 300 (Forward iteration)
    ///      - `start = 300, step = -2` -> 100 (Skips 200)
    ///      - `start = 250, step = -2` -> 100 (Rounds down to 200 first)
    ///
    ///  **Returns**
    ///
    ///  - Returns a `block` resource, `None` if block cannot be retrieved.
    fn get_block(
        &mut self, self_: wasmtime::component::Resource<Network>, start: Option<Slot>, step: i64,
    ) -> wasmtime::Result<Option<wasmtime::component::Resource<Block>>> {
        todo!()
    }

    /// Retrieve the current tips of the blockchain.
    ///
    /// **Returns**
    ///
    /// - A tuple of two slots:
    /// - The immutable tip.
    /// - The mutable tip.
    fn get_tips(
        &mut self, self_: wasmtime::component::Resource<Network>,
    ) -> wasmtime::Result<(Slot, Slot)> {
        todo!()
    }

    fn drop(&mut self, rep: wasmtime::component::Resource<Network>) -> wasmtime::Result<()> {
        todo!()
    }
}

impl HostBlock for HermesRuntimeContext {
    ///Returns whether the block is part of the immutable section of the chain.
    ///
    /// **Returns**
    ///
    /// - `true` if the block is in the immutable part.
    /// - `false` if the block is in the mutable part.
    fn is_immutable(
        &mut self, self_: wasmtime::component::Resource<Block>,
    ) -> wasmtime::Result<bool> {
        todo!()
    }

    ///  Returns whether the block is the first block of a rollback.
    ///
    /// **Returns**
    ///
    /// - `true` if the block is the first block of a rollback.
    /// - `false` if the block is not the first block of a rollback.
    fn is_rollback(
        &mut self, self_: wasmtime::component::Resource<Block>,
    ) -> wasmtime::Result<bool> {
        todo!()
    }

    /// Retrieves a transaction at the specified index within the block.
    ///
    /// **Parameters**
    ///
    /// - `index` : The index of the transaction to retrieve.
    ///
    /// **Returns**
    ///
    /// - `option<transaction>` : A `transaction` resource at the given index, `None` if the index is not found.
    fn get_txn(
        &mut self, self_: wasmtime::component::Resource<Block>, index: TxnIdx,
    ) -> wasmtime::Result<Option<wasmtime::component::Resource<Transaction>>> {
        todo!()
    }

    /// Retrieves the slot number that this block belongs to.
    ///
    /// **Returns**
    ///
    /// - `slot` : The slot number of the block.
    fn get_slot(&mut self, self_: wasmtime::component::Resource<Block>) -> wasmtime::Result<Slot> {
        todo!()
    }

    /// Returns the raw CBOR representation of the block.
    ///
    /// **Returns**
    ///
    /// - `cbor` : The CBOR format of the block.
    fn raw(&mut self, self_: wasmtime::component::Resource<Block>) -> wasmtime::Result<Cbor> {
        todo!()
    }

    fn drop(&mut self, rep: wasmtime::component::Resource<Block>) -> wasmtime::Result<()> {
        todo!()
    }
}

impl HostTransaction for HermesRuntimeContext {
    /// Returns the transaction auxiliary metadata in CBOR format.
    ///
    /// **Returns**
    ///
    /// - `cbor` : The CBOR format of the metadata.
    fn get_metadata(
        &mut self, self_: wasmtime::component::Resource<Transaction>,
    ) -> wasmtime::Result<Cbor> {
        todo!()
    }

    /// Returns the transaction hash.
    ///
    /// **Returns**
    /// - `txn-hash` : Cardano transaction hash - Blake2b-256.
    fn get_txn_hash(
        &mut self, self_: wasmtime::component::Resource<Transaction>,
    ) -> wasmtime::Result<TxnHash> {
        todo!()
    }

    /// Returns the raw CBOR representation of the transaction.
    ///
    /// **Returns**
    ///
    /// - `cbor` : The CBOR format of the transaction.
    fn raw(&mut self, self_: wasmtime::component::Resource<Transaction>) -> wasmtime::Result<Cbor> {
        todo!()
    }

    fn drop(&mut self, rep: wasmtime::component::Resource<Transaction>) -> wasmtime::Result<()> {
        todo!()
    }
}   
