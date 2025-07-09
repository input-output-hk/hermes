//! Cardano Blockchain runtime extension event handler implementation.

use crate::{
    app::ApplicationName,
    event::{HermesEvent, HermesEventPayload, TargetApp, TargetModule},
    runtime_extensions::bindings::{
        exports::hermes::cardano::event_on_block::{CardanoNetwork, SubscriptionId},
        hermes::cardano::api::Slot,
    },
    wasm::module::ModuleId,
};

/// On Cardano block event
pub(super) struct OnCardanoBlockEvent {
    /// The Cardano blockchain network that the block event is originated from.
    network: CardanoNetwork,
    /// A unique identifier of the block subscription.
    subscription_id: SubscriptionId,
    /// The Cardano slot number that the block event is in.
    slot: Slot,
    /// The CBOR format of the Cardano block data.
    block: Vec<u8>,
    /// Flag indicate whether the block is immutable or not.
    is_immutable: bool,
    /// Flag indicate whether the block is a rollback.
    is_rollback: bool,
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
    /// The Cardano blockchain network that the roll-forward event is originated from.
    network: CardanoNetwork,
    /// A unique identifier of the block subscription.
    subscription_id: SubscriptionId,
    /// The Cardano slot number that the roll-forward rolls to.
    slot: Slot,
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

// -------- Event Builder ----------

/// Build and send block event.
#[allow(clippy::too_many_arguments)]
pub(crate) fn build_and_send_block_event(
    app: ApplicationName, module_id: ModuleId, network: CardanoNetwork,
    subscription_id: SubscriptionId, slot: Slot, block_data: &[u8], is_immutable: bool,
    is_rollback: bool,
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

/// Build and send immutable roll-forward event.
pub(crate) fn build_and_send_roll_forward_event(
    app: ApplicationName, module_id: ModuleId, network: CardanoNetwork,
    subscription_id: SubscriptionId, slot: Slot,
) -> anyhow::Result<()> {
    let on_rollback_event = super::event::OnCardanoImmutableRollForwardEvent {
        network,
        subscription_id,
        slot,
    };

    crate::event::queue::send(HermesEvent::new(
        on_rollback_event,
        TargetApp::List(vec![app]),
        TargetModule::List(vec![module_id]),
    ))
}
