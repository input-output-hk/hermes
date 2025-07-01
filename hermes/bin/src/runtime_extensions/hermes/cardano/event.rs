//! Cardano Blockchain runtime extension event handler implementation.

use crate::{
    event::{HermesEvent, HermesEventPayload, TargetApp, TargetModule},
    runtime_extensions::{
        bindings::hermes::cardano::api::{Block, Network, Slot},
        hermes::cardano::{ModuleStateKey, STATE},
    },
};

/// On Cardano block event
pub(super) struct OnCardanoBlockEvent {
    /// The Cardano blockchain network that the block is originated from.
    pub(super) network: Network,
    /// The CBOR Cardano block data.
    pub(super) block: Block,
    /// The Cardano slot number that the block is in.
    pub(super) slot: Slot,
    /// Flag indicate whether the block is mutable or not.
    pub(super) is_mutable: bool,
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
                &self.block,
                self.slot,
                self.is_mutable,
            )?;
        Ok(())
    }
}

/// On Cardano rollback event
pub(super) struct OnCardanoRollbackEvent {
    /// The Cardano blockchain network where the rollback occurred.
    pub(super) network: Network,
    /// The Cardano slot number that the rollback rolls to.
    pub(super) slot: Slot,
}

impl HermesEventPayload for OnCardanoRollbackEvent {
    fn event_name(&self) -> &'static str {
        "on-cardano-rollback"
    }

    fn execute(&self, module: &mut crate::wasm::module::ModuleInstance) -> anyhow::Result<()> {
        module
            .instance
            .hermes_cardano_event_on_rollback()
            .call_on_cardano_rollback(&mut module.store, self.network, self.slot)?;

        Ok(())
    }
}

/// On Cardano roll-forward event.
pub(super) struct OnCardanoRollForwardEvent {
    /// The Cardano blockchain network that the roll-forward occurred.
    pub(super) network: Network,
    /// The Cardano slot number that the roll-forward rolls to.
    pub(super) slot: Slot,
}

impl HermesEventPayload for OnCardanoRollForwardEvent {
    fn event_name(&self) -> &'static str {
        "on-cardano-roll-forward"
    }

    fn execute(&self, module: &mut crate::wasm::module::ModuleInstance) -> anyhow::Result<()> {
        module
            .instance
            .hermes_cardano_event_on_rollback()
            .call_on_cardano_rollback(&mut module.store, self.network, self.slot)?;
        Ok(())
    }
}

/// Holds flags specifying which event subscriptions are active.
struct EventSubscriptions {
    /// Whether the module is subscribed to block events.
    blocks: bool,
    /// Whether the module is subscribed to rollback events.
    rollbacks: bool,
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
        rollbacks: sub_state.subscribed_to_rollbacks,
        roll_forwards: sub_state.subscribed_to_roll_forwards,
    })
}

// -------- Event Builder ----------

pub(crate) fn build_and_send_block_event(
    module_state_key: &ModuleStateKey, network: Network, block_data: &[u8], slot: u64,
    is_mutable: bool,
) -> anyhow::Result<()> {
    let on_block_event = super::event::OnCardanoBlockEvent {
        network,
        block: block_data.to_vec(),
        slot,
        is_mutable,
    };

    crate::event::queue::send(HermesEvent::new(
        on_block_event,
        TargetApp::List(vec![module_state_key.0.clone()]),
        TargetModule::List(vec![module_state_key.1.clone()]),
    ))
}

pub(crate) fn build_and_send_rollback_event(
    module_state_key: &ModuleStateKey, network: Network, slot: u64,
) -> anyhow::Result<()> {
    let on_rollback_event = super::event::OnCardanoRollbackEvent { network, slot };

    crate::event::queue::send(HermesEvent::new(
        on_rollback_event,
        TargetApp::List(vec![module_state_key.0.clone()]),
        TargetModule::List(vec![module_state_key.1.clone()]),
    ))
}

pub(crate) fn build_and_send_roll_forward_event(
    module_state_key: &ModuleStateKey, network: Network, slot: u64,
) -> anyhow::Result<()> {
    let on_rollback_event = super::event::OnCardanoRollForwardEvent { network, slot };

    crate::event::queue::send(HermesEvent::new(
        on_rollback_event,
        TargetApp::List(vec![module_state_key.0.clone()]),
        TargetModule::List(vec![module_state_key.1.clone()]),
    ))
}
