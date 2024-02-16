//! Init runtime extension event handler implementation.

use crate::event_queue::event::HermesEventPayload;

/// Init event
pub(crate) struct InitEvent {}

impl HermesEventPayload for InitEvent {
    fn event_name(&self) -> &str {
        "init"
    }

    fn execute(&self, module: &mut crate::wasm::module::ModuleInstance) -> anyhow::Result<()> {
        let _res = module
            .instance
            .hermes_init_event()
            .call_init(&mut module.store)?;
        Ok(())
    }
}
