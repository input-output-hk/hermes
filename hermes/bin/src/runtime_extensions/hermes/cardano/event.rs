//! Cardano Blockchain runtime extension event handler implementation.

use tracing::error;

use crate::{
    app::ApplicationName,
    event::{HermesEvent, HermesEventPayload, TargetApp, TargetModule},
    runtime_extensions::hermes::cardano::STATE,
    wasm::module::ModuleId,
};

/// On Cardano block event
pub(super) struct OnCardanoBlockEvent {
    /// An underlying 32-bit integer representation used to originally create this
    /// subscription resource of this event.
    subscription_id: u32,
    /// An underlying 32-bit integer representation used to originally create this block
    /// resource of this event.
    block: u32,
    /// Application name.
    app_name: ApplicationName,
}

impl HermesEventPayload for OnCardanoBlockEvent {
    fn event_name(&self) -> &'static str {
        "on-cardano-block"
    }

    fn execute(
        &self,
        module: &mut crate::wasm::module::ModuleInstance,
    ) -> anyhow::Result<()> {
        // Create borrow resources to send to wasm
        let subscription_id = wasmtime::component::Resource::new_borrow(self.subscription_id);
        let block = wasmtime::component::Resource::new_borrow(self.block);

        module
            .instance
            .hermes_cardano_event_on_block()
            .call_on_cardano_block(&mut module.store, subscription_id, block)?;
        Ok(())
    }
}

impl Drop for OnCardanoBlockEvent {
    fn drop(&mut self) {
        // Clean up block resource when event is dropped
        match STATE.block.get_app_state(&self.app_name) {
            Ok(block_app_state) => {
                if let Err(err) = block_app_state.delete_resource_rep(self.block) {
                    error!(error = ?err, "Failed to delete block resource OnCardanoBlockEvent");
                }
            },
            Err(err) => {
                error!(error = ?err, "Failed to get block app state OnCardanoBlockEvent");
            },
        }
    }
}

/// On Cardano roll-forward event.
pub(super) struct OnCardanoImmutableRollForwardEvent {
    /// An underlying 32-bit integer representation used to originally create this
    /// subscription resource of this event.
    subscription_id: u32,
    /// An underlying 32-bit integer representation used to originally create this block
    /// resource of this event.
    block: u32,
    /// Application name.
    app_name: ApplicationName,
}

impl HermesEventPayload for OnCardanoImmutableRollForwardEvent {
    fn event_name(&self) -> &'static str {
        "on-cardano-roll-forward"
    }

    fn execute(
        &self,
        module: &mut crate::wasm::module::ModuleInstance,
    ) -> anyhow::Result<()> {
        // Create borrow resources to send to wasm
        let subscription_id = wasmtime::component::Resource::new_borrow(self.subscription_id);
        let block = wasmtime::component::Resource::new_borrow(self.block);

        module
            .instance
            .hermes_cardano_event_on_immutable_roll_forward()
            .call_on_cardano_immutable_roll_forward(&mut module.store, subscription_id, block)?;
        Ok(())
    }
}

impl Drop for OnCardanoImmutableRollForwardEvent {
    fn drop(&mut self) {
        match STATE.block.get_app_state(&self.app_name) {
            Ok(block_app_state) => {
                if let Err(err) = block_app_state.delete_resource_rep(self.block) {
                    error!(error = ?err, "Failed to delete block resource OnCardanoImmutableRollForwardEvent");
                }
            },
            Err(err) => {
                error!(error = ?err, "Failed to get block app state OnCardanoImmutableRollForwardEvent");
            },
        }
    }
}

// -------- Event Builder ----------

/// Build and send block event.
/// Passing `subscription_id` and `block` resource 32-bit integer representation.
pub(crate) fn build_and_send_block_event(
    app_name: ApplicationName,
    module_id: ModuleId,
    subscription_id: u32,
    block: u32,
) -> anyhow::Result<()> {
    let on_block_event = super::event::OnCardanoBlockEvent {
        subscription_id,
        block,
        app_name: app_name.clone(),
    };

    crate::event::queue::send(HermesEvent::new(
        on_block_event,
        TargetApp::List(vec![app_name]),
        TargetModule::List(vec![module_id]),
    ))
}

/// Build and send immutable roll-forward event.
/// Passing `subscription_id` and `block` resource 32-bit integer representation.
pub(crate) fn build_and_send_roll_forward_event(
    app_name: ApplicationName,
    module_id: ModuleId,
    subscription_id: u32,
    block: u32,
) -> anyhow::Result<()> {
    let on_rollback_event = super::event::OnCardanoImmutableRollForwardEvent {
        subscription_id,
        block,
        app_name: app_name.clone(),
    };

    crate::event::queue::send(HermesEvent::new(
        on_rollback_event,
        TargetApp::List(vec![app_name]),
        TargetModule::List(vec![module_id]),
    ))
}
