//! Cardano Blockchain runtime extension.

use dashmap::DashMap;
use tokio::runtime::Runtime;

use crate::{
    app::ApplicationName,
    runtime_extensions::{
        bindings::{
            exports::hermes::cardano::event_on_block::SubscriptionId,
            hermes::cardano::api::{Block, Network, Transaction},
        },
        resource_manager::ApplicationResourceStorage,
    },
};

mod block;
mod chain_sync;
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
    /// Subscription ID resource storage.
    subscription_id: ApplicationResourceStorage<SubscriptionId, cardano_blockchain_types::Network>,
    /// Active subscription ID to its subscription type and network handler.
    subscriptions: DashMap<u32, (SubscriptionType, network::Handle)>,
    /// Chain sync state of a specific network.
    #[allow(clippy::struct_field_names)]
    sync_state: DashMap<cardano_blockchain_types::Network, tokio::task::JoinHandle<()>>,
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
        subscription_id: ApplicationResourceStorage::new(),
        subscriptions: DashMap::new(),
        sync_state: DashMap::new(),
    }
});

/// Advise Runtime Extensions of a new context
pub(crate) fn new_context(ctx: &crate::runtime_context::HermesRuntimeContext) {
    STATE.block.add_app(ctx.app_name().clone());
    STATE.network.add_app(ctx.app_name().clone());
    STATE.transaction.add_app(ctx.app_name().clone());
    STATE.subscription_id.add_app(ctx.app_name().clone());
}

/// Cardano Error.
#[cfg_attr(debug_assertions, derive(Debug))]
#[derive(thiserror::Error)]
pub enum CardanoError {
    /// Network not supported.
    #[error("Network {0} is not supported")]
    NetworkNotSupported(u32),
}

#[cfg(not(debug_assertions))]
impl std::fmt::Debug for CardanoError {
    fn fmt(
        &self,
        _f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        Ok(())
    }
}

/// Global multi-threaded Tokio runtime for background tasks.
#[allow(clippy::expect_used)]
pub(crate) static TOKIO_RUNTIME: once_cell::sync::Lazy<Runtime> =
    once_cell::sync::Lazy::new(|| {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("Failed to build global Tokio runtime")
    });
