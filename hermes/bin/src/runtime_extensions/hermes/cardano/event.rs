//! Cardano Blockchain runtime extension event handler implementation.

use crate::{
    app::ApplicationName,
    event::{HermesEvent, HermesEventPayload, TargetApp, TargetModule},
    runtime_extensions::{
        bindings::{
            exports::hermes::cardano::event_on_block::{CardanoNetwork, SubscriptionId},
            hermes::cardano::api::Slot,
        },
        hermes::cardano::{ModuleStateKey, STATE},
    },
    wasm::module::ModuleId,
};

/// On Cardano block event
pub(super) struct OnCardanoBlockEvent {
    /// The Cardano blockchain network that the block is originated from.
    pub(super) network: CardanoNetwork,
    /// The CBOR Cardano block data.
    pub(super) block: Vec<u8>,
    /// The Cardano slot number that the block is in.
    pub(super) slot: Slot,
    /// Flag indicate whether the block is immutable or not.
    pub(super) is_immutable: bool,
    /// Subscription ID.
    pub(super) subscription_id: SubscriptionId,
    /// Flag indicate whether the block is a rollback.
    pub(super) is_rollback: bool,
}

impl HermesEventPayload for OnCardanoBlockEvent {
    fn event_name(&self) -> &'static str {
        "on-cardano-block"
    }

    fn execute(&self, module: &mut crate::wasm::module::ModuleInstance) -> anyhow::Result<()> {
        module
            .instance
            .hermes_cardano_event_on_block()
            .call_on_cardano_block(
                &mut module.store,
                self.network,
                self.subscription_id,
                self.slot,
                &self.block,
                self.is_immutable,
                self.is_rollback,
            )?;
        Ok(())
    }
}

/// On Cardano roll-forward event.
pub(super) struct OnCardanoImmutableRollForwardEvent {
    pub(super) subscription_id: SubscriptionId,
    /// The Cardano slot number that the roll-forward rolls to.
    pub(super) slot: Slot,
    /// The Cardano blockchain network that the roll-forward occurred.
    pub(super) network: CardanoNetwork,
}

impl HermesEventPayload for OnCardanoImmutableRollForwardEvent {
    fn event_name(&self) -> &'static str {
        "on-cardano-roll-forward"
    }

    fn execute(&self, module: &mut crate::wasm::module::ModuleInstance) -> anyhow::Result<()> {
        module
            .instance
            .hermes_cardano_event_on_immutable_roll_forward()
            .call_on_cardano_immutable_roll_forward(
                &mut module.store,
                self.subscription_id,
                self.slot,
                self.network,
            )?;
        Ok(())
    }
}

/// Holds flags specifying which event subscriptions are active.
struct EventSubscriptions {
    /// Whether the module is subscribed to block events.
    blocks: bool,
    /// Whether the module is subscribed to roll-forward events.
    roll_forwards: bool,
}

/// Gets the event subscription flags for a given module.
pub(crate) fn get_event_subscriptions(
    module_state_key: &ModuleStateKey,
) -> anyhow::Result<EventSubscriptions> {
    let sub_state = STATE
        .subscriptions
        .get(module_state_key)
        .ok_or(anyhow::anyhow!("Module subscription not found"))?;

    Ok(EventSubscriptions {
        blocks: sub_state.subscribed_to_blocks,
        roll_forwards: sub_state.subscribed_to_roll_forwards,
    })
}

// -------- Event Builder ----------

pub(crate) fn build_and_send_block_event(
    app: ApplicationName, module_id: ModuleId, subscription_id: SubscriptionId,
    network: CardanoNetwork, block_data: &[u8], slot: Slot, is_immutable: bool, is_rollback: bool,
) -> anyhow::Result<()> {
    let on_block_event = super::event::OnCardanoBlockEvent {
        network,
        subscription_id,
        slot,
        block: block_data.to_vec(),
        is_immutable,
        is_rollback,
    };

    crate::event::queue::send(HermesEvent::new(
        on_block_event,
        TargetApp::List(vec![app]),
        TargetModule::List(vec![module_id]),
    ))
}

pub(crate) fn build_and_send_roll_forward_event(
    app: ApplicationName, module_id: ModuleId, subscription_id: SubscriptionId,
    network: CardanoNetwork, slot: Slot,
) -> anyhow::Result<()> {
    let on_rollback_event = super::event::OnCardanoImmutableRollForwardEvent {
        subscription_id,
        network,
        slot,
    };

    crate::event::queue::send(HermesEvent::new(
        on_rollback_event,
        TargetApp::List(vec![app]),
        TargetModule::List(vec![module_id]),
    ))
}
