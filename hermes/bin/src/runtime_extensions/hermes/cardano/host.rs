//!  Cardano Blockchain host implementation for WASM runtime.

use std::sync::atomic::{AtomicU32, Ordering};

use dashmap::DashMap;
use tracing::error;

use crate::{
    app::ApplicationName,
    runtime_context::HermesRuntimeContext,
    runtime_extensions::{
        bindings::hermes::cardano::api::{
            Block, CardanoNetwork, Cbor, CreateNetworkError, HostBlock, HostNetwork,
            HostTransaction, Network, Slot, SubscriptionId, SyncSlot, Transaction, TxnHash, TxnIdx,
        },
        hermes::cardano::network::{self, get_block_relative, get_tips, spawn_subscribe},
        resource_manager::ApplicationResourceStorage,
        utils::conversion::array_u8_32_to_tuple,
    },
};
use pallas::{codec::minicbor::to_vec, ledger::traverse::MultiEraTx};

/// Main State for Cardano blockchain runtime extension.
struct State {
    /// Network resource storage
    network: ApplicationResourceStorage<Network, cardano_blockchain_types::Network>,
    /// Lookup map for network resource
    network_lookup: DashMap<(ApplicationName, cardano_blockchain_types::Network), u32>,
    /// Block resource storage
    block: ApplicationResourceStorage<Block, cardano_blockchain_types::MultiEraBlock>,
    /// Transaction resource storage
    transaction: ApplicationResourceStorage<Transaction, MultiEraTx<'static>>,
    /// Subscription ID
    subscription_id: AtomicU32,
    /// Active subscription ID to its subscription type and chain follower handle
    subscriptions: DashMap<u32, (SubscriptionType, network::Handle)>,
}

/// Block subscription type.
#[derive(PartialEq)]
pub(crate) enum SubscriptionType {
    Block,
    ImmutableRollForward,
}

/// Initialize state
static STATE: once_cell::sync::Lazy<State> = once_cell::sync::Lazy::new(|| State {
    network: ApplicationResourceStorage::new(),
    network_lookup: DashMap::new(),
    block: ApplicationResourceStorage::new(),
    transaction: ApplicationResourceStorage::new(),
    subscription_id: AtomicU32::new(0),
    subscriptions: DashMap::new(),
});

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
        &mut self, network: CardanoNetwork,
    ) -> wasmtime::Result<Result<wasmtime::component::Resource<Network>, CreateNetworkError>> {
        let network: cardano_blockchain_types::Network = network.try_into()?;

        let key = (self.app_name().clone(), network);

        // Lookup whether the network resource already exists or not
        if let Some(rep) = STATE.network_lookup.get(&key) {
            return Ok(Ok(wasmtime::component::Resource::new_own(*rep)));
        }

        // Add app, if not yet added
        STATE.network.add_app(self.app_name().clone());
        // This should not fail, since app is added above
        let app_state = STATE.network.get_app_state(self.app_name())?;
        // Insert the network into app resource storage
        let resource = app_state.create_resource(network);
        // Store the new resource in the lookup
        STATE.network_lookup.insert(key, resource.rep());

        Ok(Ok(resource))
    }

    /// Subscribe to blockchain block events, start from a specified starting point.
    ///
    /// This sets up a subscription to receive new block and block rollback updates starting from the
    /// given `start`.
    ///
    /// **Parameters**
    ///
    /// - `start`: The slot to begin following from.
    ///
    /// **Returns**
    ///
    /// - `subscription-id`: A unique identifier of this subscription.
    ///                         Use to distinguishes events from different subscribers
    ///                         and provides control over subscription management.
    ///                         The ID must be unique across all active subscriptions.
    fn subscribe_block(
        &mut self, self_: wasmtime::component::Resource<Network>, start: SyncSlot,
    ) -> wasmtime::Result<SubscriptionId> {
        let network = STATE
            .network
            .get_app_state(self.app_name())?
            .get_object(&self_)?;

        let id = STATE.subscription_id.fetch_add(1, Ordering::Relaxed);
        let handle = spawn_subscribe(
            self.app_name().clone(),
            self.module_id().clone(),
            start.into(),
            *network,
            SubscriptionType::Block,
            id,
        );
        STATE
            .subscriptions
            .insert(id, (SubscriptionType::Block, handle));

        Ok(SubscriptionId::from(id))
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
    /// - `subscription-id`: A unique identifier of this subscription.
    ///                         Use to distinguishes events from different subscribers
    ///                         and provides control over subscription management.
    ///                         The ID must be unique across all active subscriptions.
    fn subscribe_immutable_roll_forward(
        &mut self, self_: wasmtime::component::Resource<Network>, start: SyncSlot,
    ) -> wasmtime::Result<SubscriptionId> {
        let network = STATE
            .network
            .get_app_state(self.app_name())?
            .get_object(&self_)?;

        let id = STATE.subscription_id.fetch_add(1, Ordering::Relaxed);

        let handle = spawn_subscribe(
            self.app_name().clone(),
            self.module_id().clone(),
            start.into(),
            *network,
            SubscriptionType::ImmutableRollForward,
            id,
        );
        STATE
            .subscriptions
            .insert(id, (SubscriptionType::Block, handle));
        Ok(SubscriptionId::from(id))
    }

    /// Unsubscribing block events given an ID.
    /// Once this function is called, the subscription instance, `subscription-id` will be removed.
    ///
    /// **Parameters**
    /// - `id` : A unique identifier of the block subscription to unsubscribe from.
    ///         This `id` is returned from the `subscribe-block` or `subscribe-immutable-roll-forward`
    fn unsubscribe(&mut self, id: SubscriptionId) -> wasmtime::Result<()> {
        if let Some(entry) = STATE.subscriptions.get(&id) {
            let (_, handle) = entry.value();
            handle.stop();
            STATE.subscriptions.remove(&id);
        }
        Ok(())
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
        let mut network_state = STATE.network.get_app_state(self.app_name())?;
        let network = network_state.get_object(&self_)?;
        let multi_era_block = get_block_relative(*network, start, step).map_err(|e| {
            error!("get_block: {e}");
            e
        })?;
        STATE.block.add_app(self.app_name().clone());
        // Insert the block into app resource storage
        let app_state = STATE.block.get_app_state(self.app_name())?;
        let resource = app_state.create_resource(multi_era_block);
        Ok(Some(resource))
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
        let mut binding = STATE.network.get_app_state(self.app_name())?;
        let network = binding.get_object(&self_)?;
        let (immutable_tip, mutable_tip) = get_tips(*network)?;
        Ok((immutable_tip.into(), mutable_tip.into()))
    }

    fn drop(&mut self, rep: wasmtime::component::Resource<Network>) -> wasmtime::Result<()> {
        let mut app_state = STATE.network.get_app_state(self.app_name())?;
        let network = app_state.get_object(&rep)?;
        app_state.delete_resource(rep)?;
        let key = (self.app_name().clone(), *network);
        STATE.network_lookup.remove(&key);
        Ok(())
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
        let mut block_state = STATE.block.get_app_state(self.app_name())?;
        let block = block_state.get_object(&self_)?;
        Ok(block.is_immutable())
    }

    /// Returns whether the block is the first block of a rollback.
    ///
    /// **Returns**
    ///
    /// - `true` if the block is the first block of a rollback.
    /// - `false` if the block is not the first block of a rollback.
    fn is_rollback(
        &mut self, self_: wasmtime::component::Resource<Block>,
    ) -> wasmtime::Result<bool> {
        let mut block_state = STATE.block.get_app_state(self.app_name())?;
        let block = block_state.get_object(&self_)?;
        // FIXME: where to get rollback?
        Ok()
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
        let mut block_state = STATE.block.get_app_state(self.app_name())?;
        let block = block_state.get_object(&self_)?;

        // Insert the tx into app resource storage
        let binding = block.txs();
        let txn = binding
            .get(index as usize)
            .ok_or_else(|| anyhow::anyhow!("Transaction at index not found"))?;
        let mut txn_state = STATE.transaction.get_app_state(self.app_name())?;
        let resource = txn_state.create_resource(txn);
        Ok(Some(resource))
    }

    /// Retrieves the slot number that this block belongs to.
    ///
    /// **Returns**
    ///
    /// - `slot` : The slot number of the block.
    fn get_slot(&mut self, self_: wasmtime::component::Resource<Block>) -> wasmtime::Result<Slot> {
        let mut block_state = STATE.block.get_app_state(self.app_name())?;
        let block = block_state.get_object(&self_)?;
        Ok(block.slot().into())
    }

    /// Returns the raw CBOR representation of the block.
    ///
    /// **Returns**
    ///
    /// - `cbor` : The CBOR format of the block.
    fn raw(&mut self, self_: wasmtime::component::Resource<Block>) -> wasmtime::Result<Cbor> {
        let mut block_state = STATE.block.get_app_state(self.app_name())?;
        let block = block_state.get_object(&self_)?;
        Ok(block.raw().clone().into())
    }

    fn drop(&mut self, rep: wasmtime::component::Resource<Block>) -> wasmtime::Result<()> {
        let app_state = STATE.block.get_app_state(self.app_name())?;
        app_state.delete_resource(rep)?;
        Ok(())
    }
}

impl HostTransaction for HermesRuntimeContext {
    /// Returns the transaction auxiliary data in CBOR format.
    ///
    /// **Returns**
    ///
    /// - `cbor` : The CBOR format of the auxiliary data.
    fn get_aux(
        &mut self, self_: wasmtime::component::Resource<Transaction>,
    ) -> wasmtime::Result<Cbor> {
        let mut tx_state = STATE.transaction.get_app_state(self.app_name())?;
        let tx = tx_state.get_object(&self_)?;
        let metadata = tx
            .metadata()
            .as_alonzo()?;
        let bytes = to_vec(metadata)?;
        Ok(bytes)
    }

    /// Returns the transaction hash.
    ///
    /// **Returns**
    /// - `txn-hash` : Cardano transaction hash - Blake2b-256.
    fn get_txn_hash(
        &mut self, self_: wasmtime::component::Resource<Transaction>,
    ) -> wasmtime::Result<TxnHash> {
        let mut tx_state = STATE.transaction.get_app_state(self.app_name())?;
        let tx = tx_state.get_object(&self_)?;
        let slice: [u8; 32] =
            tx.hash().as_ref().try_into().map_err(|_| {
                anyhow::anyhow!("Expected 32 bytes, got {}", tx.hash().as_ref().len())
            })?;
        Ok(array_u8_32_to_tuple(&slice))
    }

    /// Returns the raw CBOR representation of the transaction.
    ///
    /// **Returns**
    ///
    /// - `cbor` : The CBOR format of the transaction.
    fn raw(&mut self, self_: wasmtime::component::Resource<Transaction>) -> wasmtime::Result<Cbor> {
        let mut tx_state = STATE.transaction.get_app_state(self.app_name())?;
        let tx = tx_state.get_object(&self_)?;
        Ok(tx.encode())
    }

    fn drop(&mut self, rep: wasmtime::component::Resource<Transaction>) -> wasmtime::Result<()> {
        let tx_state = STATE.transaction.get_app_state(self.app_name())?;
        tx_state.delete_resource(rep)?;
        Ok(())
    }
}
