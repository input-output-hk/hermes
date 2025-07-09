//! Cardano Blockchain runtime extension.

use std::sync::atomic::AtomicU32;

use dashmap::DashMap;

use crate::{
    app::ApplicationName,
    runtime_extensions::{
        bindings::hermes::cardano::api::{Block, Network, Transaction},
        resource_manager::ApplicationResourceStorage,
    },
};

mod block;
mod event;
mod host;
mod network;

/// Main State for Cardano blockchain runtime extension.
struct State {
    /// Network resource storage.
    network: ApplicationResourceStorage<Network, cardano_blockchain_types::Network>,
    /// Lookup map for network resource.
    network_lookup: DashMap<(ApplicationName, cardano_blockchain_types::Network), u32>,
    /// Block resource storage.
    block: ApplicationResourceStorage<Block, cardano_blockchain_types::MultiEraBlock>,
    /// Transaction resource storage.
    // Use (MultiEraBlock, slot) instead of pallas::ledger::traverse::MultiEraTx due to life time
    // issue
    transaction:
        ApplicationResourceStorage<Transaction, (cardano_blockchain_types::MultiEraBlock, u16)>,
    /// Subscription ID.
    subscription_id: AtomicU32,
    /// Active subscription ID to its subscription type and network handler.
    subscriptions: DashMap<u32, (SubscriptionType, network::Handle)>,
}

/// Block subscription type.
#[derive(PartialEq)]
pub(crate) enum SubscriptionType {
    /// Normal block subscription.
    Block,
    /// Immutable roll forward block subscription.
    ImmutableRollForward,
}

/// Initialize state
static STATE: once_cell::sync::Lazy<State> = once_cell::sync::Lazy::new(|| {
    State {
        network: ApplicationResourceStorage::new(),
        network_lookup: DashMap::new(),
        block: ApplicationResourceStorage::new(),
        transaction: ApplicationResourceStorage::new(),
        subscription_id: AtomicU32::new(0),
        subscriptions: DashMap::new(),
    }
});

/// Advise Runtime Extensions of a new context
pub(crate) fn new_context(ctx: &crate::runtime_context::HermesRuntimeContext) {
    STATE.block.add_app(ctx.app_name().clone());
    STATE.network.add_app(ctx.app_name().clone());
    STATE.transaction.add_app(ctx.app_name().clone());
}

/// Cardano Error.
#[derive(thiserror::Error, Debug)]
pub enum CardanoError {
    /// Network not supported.
    #[error("Network {0} is not supported")]
    NetworkNotSupported(u32),
}
