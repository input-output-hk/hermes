//! Cardano Blockchain runtime extension implementation.

use cardano_chain_follower::PointOrTip;
use dashmap::DashMap;

use crate::{
    app::HermesAppName,
    runtime_extensions::bindings::hermes::cardano::api::{CardanoBlockchainId, SubscribeOptions},
    wasm::module::ModuleId,
};

mod chain_follower_task;
mod event;
mod host;
mod tokio_runtime_task;

/// Cardano Runtime Extension internal result type.
pub(super) type Result<T> = anyhow::Result<T>;

/// Hermes application module subscription state.
#[derive(Default)]
struct SubscriptionState {
    /// Whether the module is subscribed to receive block events.
    subscribed_to_blocks: bool,
    /// Whether the module is subscribed to receive transaction events.
    subscribed_to_txns: bool,
    /// Handle to the cardano chain follower from which the module is receiving
    /// events.
    follower_handle: Option<chain_follower_task::Handle>,
    /// Current slot that the subscription is at.
    current_slot: u64,
}

/// Triple representing the key of the subscription state map.
type ModuleStateKey = (HermesAppName, ModuleId, cardano_chain_follower::Network);

/// Cardano Runtime Extension state.
struct State {
    /// Handle to the Tokio runtime background thread.
    tokio_rt_handle: tokio_runtime_task::Handle,
    /// Mapping of application module subscription states.
    subscriptions: DashMap<ModuleStateKey, SubscriptionState>,
    /// Chain followers configured only for reading blocks.
    readers: DashMap<cardano_chain_follower::Network, cardano_chain_follower::Follower>,
}

/// Cardano Runtime Extension internal state.
static STATE: once_cell::sync::Lazy<State> = once_cell::sync::Lazy::new(|| {
    // Spawn a thread for running a Tokio runtime if we haven't yet.
    // This is done so we can run async functions.
    let tokio_rt_handle = tokio_runtime_task::spawn();

    State {
        tokio_rt_handle,
        subscriptions: DashMap::new(),
        readers: DashMap::new(),
    }
});

/// Advise Runtime Extensions of a new context
pub(crate) fn new_context(_ctx: &crate::runtime_context::HermesRuntimeContext) {}

/// Subscribes a module or resumes the generation of subscribed events for a module.
pub(super) fn subscribe(
    chain_id: CardanoBlockchainId, app_name: HermesAppName, module_id: ModuleId,
    whence: Option<PointOrTip>, what: SubscribeOptions,
) -> Result<()> {
    let network = chain_id.into();

    let mut sub_state = STATE
        .subscriptions
        .entry((app_name.clone(), module_id.clone(), network))
        .or_default();

    // Set what we want to subscribe to.
    if what.contains(SubscribeOptions::BLOCK) {
        sub_state.subscribed_to_blocks = true;
    }

    if what.contains(SubscribeOptions::TRANSACTION) {
        sub_state.subscribed_to_txns = true;
    }

    if let Some(follow_from) = whence {
        if let Some(handle) = sub_state.follower_handle.as_ref() {
            handle.set_read_pointer_sync(follow_from)?;
        } else {
            let (follower_handle, starting_point) = STATE.tokio_rt_handle.spawn_follower_sync(
                app_name,
                module_id,
                chain_id,
                follow_from,
            )?;

            sub_state.follower_handle = Some(follower_handle);
            sub_state.current_slot = starting_point.slot_or_default();
        }
    } else if let Some(handle) = sub_state.follower_handle.as_ref() {
        handle.resume()?;
    }

    Ok(())
}

/// Unsubscribes a module or stops the generation of subscribed events for a module.
pub(super) fn unsubscribe(
    chain_id: CardanoBlockchainId, app_name: HermesAppName, module_id: ModuleId,
    opts: SubscribeOptions,
) -> Result<()> {
    let network = chain_id.into();
    let sub_state = STATE.subscriptions.get_mut(&(app_name, module_id, network));

    if let Some(mut sub_state) = sub_state {
        let mut block_stopped = false;
        let mut txn_stopped = false;

        if opts.contains(SubscribeOptions::BLOCK) && sub_state.subscribed_to_blocks {
            sub_state.subscribed_to_blocks = false;
            block_stopped = true;
        }

        if opts.contains(SubscribeOptions::TRANSACTION) && sub_state.subscribed_to_txns {
            sub_state.subscribed_to_txns = false;
            txn_stopped = true;
        }

        // If we changed the subscription state, and ended up subscribed to nothing, then just STOP.
        if (block_stopped || txn_stopped)
            && !sub_state.subscribed_to_blocks
            && !sub_state.subscribed_to_txns
        {
            if let Some(handle) = sub_state.follower_handle.as_ref() {
                handle.stop()?;
            }
        }
    }

    Ok(())
}

/// Reads a block from a Cardano network.
pub(super) fn read_block(
    chain_id: CardanoBlockchainId, at: cardano_chain_follower::PointOrTip,
) -> Result<cardano_chain_follower::MultiEraBlockData> {
    STATE.tokio_rt_handle.read_block(chain_id, at)
}

impl From<CardanoBlockchainId> for cardano_chain_follower::Network {
    fn from(chain_id: CardanoBlockchainId) -> Self {
        match chain_id {
            CardanoBlockchainId::Mainnet => cardano_chain_follower::Network::Mainnet,
            CardanoBlockchainId::Preprod => cardano_chain_follower::Network::Preprod,
            CardanoBlockchainId::Preview => cardano_chain_follower::Network::Preview,
        }
    }
}

#[cfg(test)]
mod test {
    use cardano_chain_follower::PointOrTip;

    use super::{read_block, subscribe, unsubscribe};
    use crate::{
        app::HermesAppName,
        runtime_extensions::bindings::hermes::cardano::api::{
            CardanoBlockchainId, SubscribeOptions,
        },
    };

    #[test]
    #[ignore = "Just for testing locally"]
    fn subscription_works() {
        let app_name = HermesAppName("test_app_it_works".to_string());
        let module_id = crate::wasm::module::ModuleId(rusty_ulid::Ulid::generate());

        subscribe(
            CardanoBlockchainId::Preprod,
            app_name.clone(),
            module_id.clone(),
            Some(PointOrTip::Tip),
            SubscribeOptions::BLOCK,
        )
        .expect("subscribed");

        subscribe(
            CardanoBlockchainId::Preprod,
            app_name.clone(),
            module_id.clone(),
            Some(PointOrTip::Tip),
            SubscribeOptions::TRANSACTION,
        )
        .expect("subscribed");

        subscribe(
            CardanoBlockchainId::Preprod,
            app_name.clone(),
            module_id.clone(),
            Some(PointOrTip::Tip),
            SubscribeOptions::all(),
        )
        .expect("subscribed");

        subscribe(
            CardanoBlockchainId::Preprod,
            app_name.clone(),
            module_id.clone(),
            Some(PointOrTip::Tip),
            SubscribeOptions::empty(),
        )
        .expect("subscribed");

        std::thread::sleep(std::time::Duration::from_secs(5));

        subscribe(
            CardanoBlockchainId::Preprod,
            app_name.clone(),
            module_id.clone(),
            Some(PointOrTip::Point(cardano_chain_follower::Point::Specific(
                49_075_522,
                hex::decode("b7639b523f320643236ab0fc04b7fd381dedd42c8d6b6433b5965a5062411396")
                    .expect("decode hex value"),
            ))),
            SubscribeOptions::empty(),
        )
        .expect("subscribed");

        std::thread::sleep(std::time::Duration::from_secs(5));

        unsubscribe(
            CardanoBlockchainId::Preprod,
            app_name.clone(),
            module_id.clone(),
            SubscribeOptions::BLOCK,
        )
        .expect("subscribed");

        std::thread::sleep(std::time::Duration::from_secs(5));

        unsubscribe(
            CardanoBlockchainId::Preprod,
            app_name.clone(),
            module_id.clone(),
            SubscribeOptions::all(),
        )
        .expect("unsubscribed");

        std::thread::sleep(std::time::Duration::from_secs(5));

        subscribe(
            CardanoBlockchainId::Preprod,
            app_name.clone(),
            module_id.clone(),
            None,
            SubscribeOptions::empty(),
        )
        .expect("subscribed");

        std::thread::sleep(std::time::Duration::from_secs(100));
    }

    #[test]
    #[ignore = "Just for local testing"]
    fn reading_works() {
        let block_data = read_block(
            CardanoBlockchainId::Preprod,
            cardano_chain_follower::Point::Specific(
                49_075_522,
                hex::decode("b7639b523f320643236ab0fc04b7fd381dedd42c8d6b6433b5965a5062411396")
                    .expect("decode hex value"),
            )
            .into(),
        )
        .expect("read");

        assert_eq!(block_data.decode().slot(), 49_075_522);
    }
}
