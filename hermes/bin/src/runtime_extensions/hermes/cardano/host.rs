//!  Cardano Blockchain host implementation for WASM runtime.

use tracing::error;

use crate::{
    runtime_context::HermesRuntimeContext,
    runtime_extensions::{
        bindings::hermes::cardano::api::{
            Block, BlockError, CardanoNetwork, Cbor, CreateNetworkError, Host, HostBlock,
            HostNetwork, HostSubscriptionId, HostTransaction, Network, Slot, SubscribeError,
            SubscriptionId, SyncSlot, Transaction, TransactionError, TxnHash, TxnIdx,
        },
        hermes::cardano::{
            STATE, SubscriptionType,
            block::{get_block_relative, get_is_rollback, get_tips},
            chain_sync::spawn_chain_sync_task,
            network::{spawn_subscribe, sync_slot_to_point},
        },
        utils::conversion::array_u8_32_to_tuple,
    },
};

impl HostNetwork for HermesRuntimeContext {
    /// Create a new Cardano network resource instance.
    ///
    /// **Parameters**
    //
    /// - `network`: The Cardano network to connect to (e.g., Mainnet, Preprod, Preview).
    ///
    /// **Returns**
    ///
    /// - `ok(network)`: A resource network, if successfully create network resource.
    /// - `error(create-network-error)`: If creating network resource failed.
    fn new(
        &mut self,
        network: CardanoNetwork,
    ) -> wasmtime::Result<Result<wasmtime::component::Resource<Network>, CreateNetworkError>> {
        let network: cardano_blockchain_types::Network = match network.try_into() {
            Ok(n) => n,
            Err(_) => {
                return Ok(Err(CreateNetworkError::NetworkNotSupported));
            },
        };
        let key = (self.app_name().clone(), network);
        // Lookup whether the network resource already exists
        if let Some(rep) = STATE.network_lookup.get(&key) {
            return Ok(Ok(wasmtime::component::Resource::new_own(*rep)));
        }

        // If not, create a new resource
        let app_state = STATE.network.get_app_state(self.app_name())?;
        let resource = app_state.create_resource(network);
        // Store the new resource in the lookup
        STATE.network_lookup.insert(key, resource.rep());

        // Spawn a chain sync task, which is required before following
        spawn_chain_sync_task(network);

        Ok(Ok(resource))
    }

    /// Subscribe to blockchain block events, start from a specified starting point.
    ///
    /// This sets up a subscription to receive new block and block rollback updates
    /// starting from the given `start`.
    ///
    /// **Parameters**
    ///
    /// - `start`: The slot to begin following from.
    ///
    /// **Returns**
    ///
    /// - `ok(u32)`: A unsigned integer represent the underlying 32-bit representation of
    ///   subscription ID resource. this subscription. Use to distinguishes events from
    ///   different subscribers and provides control over subscription management.The ID
    ///   must be unique across all active subscriptions.
    /// - `error(subscribe-error)`: If subscription failed.
    fn subscribe_block(
        &mut self,
        self_: wasmtime::component::Resource<Network>,
        start: SyncSlot,
    ) -> wasmtime::Result<Result<u32, SubscribeError>> {
        let network_app_state = STATE.network.get_app_state_readonly(self.app_name())?;
        let network = network_app_state.get_object_shared(&self_)?;

        let subscription_id_app_state = STATE.subscription_id.get_app_state(self.app_name())?;
        let subscription_id_resource = subscription_id_app_state.create_resource(*network);
        let borrow_subscription_id =
            wasmtime::component::Resource::new_borrow(subscription_id_resource.rep());

        let start = sync_slot_to_point(start, *network);
        let handle = spawn_subscribe(
            self.app_name().clone(),
            self.module_id().clone(),
            start,
            *network,
            SubscriptionType::Block,
            borrow_subscription_id,
        );
        STATE.subscriptions.insert(
            subscription_id_resource.rep(),
            (SubscriptionType::Block, handle),
        );
        // Return representative instead of the actual resource, so the resource won't drop
        // when the event is trigger.
        Ok(Ok(subscription_id_resource.rep()))
    }

    /// Subscribe to blockchain immutable rolls forward.
    ///
    /// This sets up a subscription to receive event when the immutable part of the
    /// blockchain roll forwards.
    ///
    /// **Parameters**
    ///
    /// - `start`: The slot to begin following from.
    ///
    /// **Returns**
    ///
    /// - `ok(u32)`: A unsigned integer represent the underlying 32-bit representation of
    ///   subscription ID resource. this subscription. Use to distinguishes events from
    ///   different subscribers and provides control over subscription management.The ID
    ///   must be unique across all active subscriptions.
    /// - `error(subscribe-error)`: If subscription failed.
    fn subscribe_immutable_roll_forward(
        &mut self,
        self_: wasmtime::component::Resource<Network>,
        start: SyncSlot,
    ) -> wasmtime::Result<Result<u32, SubscribeError>> {
        let network_app_state = STATE.network.get_app_state_readonly(self.app_name())?;
        let network = network_app_state.get_object_shared(&self_)?;

        let subscription_id_app_state = STATE.subscription_id.get_app_state(self.app_name())?;
        let subscription_id_resource = subscription_id_app_state.create_resource(*network);
        let borrow_subscription_id =
            wasmtime::component::Resource::new_borrow(subscription_id_resource.rep());

        let start = sync_slot_to_point(start, *network);
        let handle = spawn_subscribe(
            self.app_name().clone(),
            self.module_id().clone(),
            start,
            *network,
            SubscriptionType::ImmutableRollForward,
            borrow_subscription_id,
        );
        STATE.subscriptions.insert(
            subscription_id_resource.rep(),
            (SubscriptionType::ImmutableRollForward, handle),
        );
        // Return representative instead of the actual resource, so the resource won't drop
        // when the event is trigger.
        Ok(Ok(subscription_id_resource.rep()))
    }

    #[allow(clippy::doc_lazy_continuation)]
    /// Get a block relative to `start` by `step`.
    ///
    /// **Parameters**
    ///  - `start`: Slot to begin retrieval from, current tip if `None`.
    ///  - `step` -`0` : the block at `start`, will return `None` if there is no block
    ///    exactly at this `start` slot. -`+n`: the `n`‑th block *after* the given `start`
    ///    slot. –`‑n`: the `n`‑th block *before* the given `start` slot.
    ///
    ///    Note: For both `+n` and `-n`, the `start` does not need to be a true block.
    ///    They will return the block which appears at this block offset, given the
    /// arbitrary start point.  IF the `start` block does exist, it will never
    /// returned with a positive or negative `step`, as it is `step` 0.
    ///
    /// Example, Given three consecutive blocks at slots `100`, `200` and `300` the
    /// following will be returned:
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
        &mut self,
        self_: wasmtime::component::Resource<Network>,
        start: Option<Slot>,
        step: i64,
    ) -> wasmtime::Result<Option<wasmtime::component::Resource<Block>>> {
        let app_state = STATE.network.get_app_state_readonly(self.app_name())?;
        let network = app_state.get_object_shared(&self_)?;
        let multi_era_block = match get_block_relative(*network, start, step) {
            Ok(block) => block,
            Err(e) => {
                error!(error=?e, "Failed to get block");
                return Ok(None);
            },
        };
        let app_state = STATE.block.get_app_state_readonly(self.app_name())?;
        let resource = app_state.create_resource(multi_era_block);
        Ok(Some(resource))
    }

    /// Retrieve the current tips of the blockchain.
    ///
    /// **Returns**
    ///
    /// - A tuple of two slots:
    ///     - The immutable tip.
    ///     - The mutable tip. `None` if the tips cannot be retrieved.
    fn get_tips(
        &mut self,
        self_: wasmtime::component::Resource<Network>,
    ) -> wasmtime::Result<Option<(Slot, Slot)>> {
        let app_state = STATE.network.get_app_state_readonly(self.app_name())?;
        let network = app_state.get_object_shared(&self_)?;
        let (immutable_tip, mutable_tip) = match get_tips(*network) {
            Ok(tips) => tips,
            Err(e) => {
                error!(error=?e, "Failed to get tips");
                return Ok(None);
            },
        };
        Ok(Some((immutable_tip.into(), mutable_tip.into())))
    }

    fn drop(
        &mut self,
        rep: wasmtime::component::Resource<Network>,
    ) -> wasmtime::Result<()> {
        // Remove from resource manager
        let mut app_state = STATE.network.get_app_state(self.app_name())?;
        let network = *app_state.get_object(&rep)?;
        app_state.delete_resource(rep)?;
        // Remove from lookup
        let key = (self.app_name().clone(), network);
        STATE.network_lookup.remove(&key);
        Ok(())
    }
}

impl HostBlock for HermesRuntimeContext {
    /// Returns whether the block is part of the immutable section of the chain.
    ///
    /// **Returns**
    ///
    /// - `true` if the block is in the immutable part.
    /// - `false` if the block is in the mutable part.
    fn is_immutable(
        &mut self,
        self_: wasmtime::component::Resource<Block>,
    ) -> wasmtime::Result<bool> {
        let app_state = STATE.block.get_app_state_readonly(self.app_name())?;
        let block = app_state.get_object_shared(&self_)?;
        Ok(block.is_immutable())
    }

    /// Returns whether the block is the first block of a rollback.
    ///
    /// **Returns**
    ///
    /// - `ok(bool)` True if the block is the first block of a rollback, otherwise, False.
    /// - `error(block-error)`: If block cannot be retrieved.
    fn is_rollback(
        &mut self,
        self_: wasmtime::component::Resource<Block>,
    ) -> wasmtime::Result<Result<bool, BlockError>> {
        let app_state = STATE.block.get_app_state_readonly(self.app_name())?;
        let block = app_state.get_object_shared(&self_)?;
        let is_rollback = get_is_rollback(block.network(), block.slot())?;
        match is_rollback {
            Some(is_rollback) => Ok(Ok(is_rollback)),
            None => Ok(Err(BlockError::BlockNotFound)),
        }
    }

    /// Retrieves a transaction at the specified index within the block.
    ///
    /// **Parameters**
    ///
    /// - `index` : The index of the transaction to retrieve.
    ///
    /// **Returns**
    ///
    /// - `ok(transaction)` : A `transaction` resource at the given index
    /// - `error(transaction-error)`: If a transaction data does not exist in the block at
    ///   the given index.
    fn get_txn(
        &mut self,
        self_: wasmtime::component::Resource<Block>,
        index: TxnIdx,
    ) -> wasmtime::Result<Result<wasmtime::component::Resource<Transaction>, TransactionError>>
    {
        let app_state = STATE.block.get_app_state_readonly(self.app_name())?;
        let block = app_state.get_object_shared(&self_)?;
        // Check whether the data in the index exists
        if block.txs().get(usize::from(index)).is_none() {
            return Ok(Err(TransactionError::TxnNotFound));
        }
        // If exist store the block and index
        let app_state = STATE.transaction.get_app_state_readonly(self.app_name())?;
        let resource = app_state.create_resource((block.clone(), index));
        Ok(Ok(resource))
    }

    /// Retrieves the slot number that this block belongs to.
    ///
    /// **Returns**
    ///
    /// - `slot` : The slot number of the block.
    fn get_slot(
        &mut self,
        self_: wasmtime::component::Resource<Block>,
    ) -> wasmtime::Result<Slot> {
        let app_state = STATE.block.get_app_state_readonly(self.app_name())?;
        let block = app_state.get_object_shared(&self_)?;
        Ok(block.slot().into())
    }

    /// Returns the raw CBOR representation of the block.
    ///
    /// **Returns**
    ///
    /// - `cbor` : The CBOR format of the block.
    fn raw(
        &mut self,
        self_: wasmtime::component::Resource<Block>,
    ) -> wasmtime::Result<Cbor> {
        let app_state = STATE.block.get_app_state_readonly(self.app_name())?;
        let block = app_state.get_object_shared(&self_)?;
        Ok(block.raw().clone())
    }

    /// Fork count is a counter that is incremented every time there is a roll-back in
    /// live-chain. It is used to help followers determine how far to roll-back to
    /// resynchronize without storing full block history. The fork count starts at 1 for
    /// live blocks and increments if the live chain tip is purged due to a detected
    /// fork, but it does not track the exact number of forks reported by peers.
    ///
    /// - 0 - for all immutable data
    /// - 1 - for any data read from the blockchain during a *backfill* on initial sync
    /// - 2+ - for each subsequent rollback detected while reading live blocks.
    ///
    /// ** Returns **
    ///
    /// - `u64` : The fork count.
    fn get_fork(
        &mut self,
        self_: wasmtime::component::Resource<Block>,
    ) -> wasmtime::Result<u64> {
        let app_state = STATE.block.get_app_state_readonly(self.app_name())?;
        let block = app_state.get_object_shared(&self_)?;
        Ok(block.fork().into())
    }

    fn drop(
        &mut self,
        rep: wasmtime::component::Resource<Block>,
    ) -> wasmtime::Result<()> {
        let app_state = STATE.block.get_app_state(self.app_name())?;
        app_state.delete_resource(rep)?;
        Ok(())
    }
}

impl HostTransaction for HermesRuntimeContext {
    /// Returns the transaction auxiliary metadata in CBOR format.
    ///
    /// **Parameters**
    ///
    /// - `label`: A metadata label used as a key to get the associated metadata.
    ///
    /// **Returns**
    ///
    /// - `option<cbor>` : The CBOR format of the metadata, `None` if the label requested
    ///   is not present.
    fn get_metadata(
        &mut self,
        self_: wasmtime::component::Resource<Transaction>,
        label: u64,
    ) -> wasmtime::Result<Option<Cbor>> {
        let app_state = STATE.transaction.get_app_state_readonly(self.app_name())?;
        let object = app_state.get_object_shared(&self_)?;
        let Some(metadata) = object.0.txn_metadata(object.1.into(), label.into()) else {
            error!(
                "Failed to get metadata, transaction index: {}, label: {label}",
                object.1
            );
            return Ok(None);
        };
        Ok(Some(metadata.as_ref().to_vec()))
    }

    /// Returns the transaction hash.
    ///
    /// **Returns**
    ///
    /// - `option<txn-hash>` : Cardano transaction hash - Blake2b-256, `None` if cannot
    ///   retrieve the transaction hash.
    fn get_txn_hash(
        &mut self,
        self_: wasmtime::component::Resource<Transaction>,
    ) -> wasmtime::Result<Option<TxnHash>> {
        let app_state = STATE.transaction.get_app_state_readonly(self.app_name())?;
        let object = app_state.get_object_shared(&self_)?;
        let txns = object.0.txs();
        let Some(txn) = txns.get(usize::from(object.1)) else {
            error!(error = "Invalid index", "Failed to get transaction hash");
            return Ok(None);
        };
        let slice: [u8; 32] = match txn.hash().as_ref().try_into() {
            Ok(arr) => arr,
            Err(_) => return Ok(None),
        };
        Ok(Some(array_u8_32_to_tuple(&slice)))
    }

    /// Returns the raw CBOR representation of the transaction.
    ///
    /// **Returns**
    ///
    /// - `option<cbor>` : The CBOR format of the transaction, `None` if cannot retrieve
    ///   the raw transaction.
    fn raw(
        &mut self,
        self_: wasmtime::component::Resource<Transaction>,
    ) -> wasmtime::Result<Option<Cbor>> {
        let app_state = STATE.transaction.get_app_state_readonly(self.app_name())?;
        let object = app_state.get_object_shared(&self_)?;
        let txns = object.0.txs();
        let Some(txn) = txns.get(usize::from(object.1)) else {
            error!(error = "Invalid index", "Failed to get raw transaction");
            return Ok(None);
        };
        Ok(Some(txn.encode()))
    }

    fn drop(
        &mut self,
        rep: wasmtime::component::Resource<Transaction>,
    ) -> wasmtime::Result<()> {
        let app_state = STATE.transaction.get_app_state(self.app_name())?;
        app_state.delete_resource(rep)?;
        Ok(())
    }
}

impl HostSubscriptionId for HermesRuntimeContext {
    /// Returns the network that this subscription is in.
    ///
    /// **Returns**
    // - `cardano-network` : The Cardano network that this subscription is in.
    fn get_network(
        &mut self,
        self_: wasmtime::component::Resource<SubscriptionId>,
    ) -> wasmtime::Result<CardanoNetwork> {
        let app_state = STATE
            .subscription_id
            .get_app_state_readonly(self.app_name())?;
        let network = app_state.get_object_shared(&self_)?;
        Ok((*network).try_into()?)
    }

    /// Unsubscribing block event of this `subscription-id` instance.
    /// Once this function is called, the subscription instance, `subscription-id` will be
    /// removed.
    fn unsubscribe(
        &mut self,
        self_: wasmtime::component::Resource<SubscriptionId>,
    ) -> wasmtime::Result<()> {
        let id = self_.rep();
        if let Some(entry) = STATE.subscriptions.get(&id) {
            let (_, handle) = entry.value();
            // Stop the subscription
            handle.stop();
            // Remove the resource and subscription state
            let subscription_id_app_state = STATE.subscription_id.get_app_state(self.app_name())?;
            subscription_id_app_state.delete_resource(self_)?;
            STATE.subscriptions.remove(&id);
        }
        Ok(())
    }

    fn drop(
        &mut self,
        rep: wasmtime::component::Resource<SubscriptionId>,
    ) -> wasmtime::Result<()> {
        let app_state = STATE.subscription_id.get_app_state(self.app_name())?;
        STATE.subscriptions.remove(&rep.rep());
        app_state.delete_resource(rep)?;
        Ok(())
    }
}

impl Host for HermesRuntimeContext {}
