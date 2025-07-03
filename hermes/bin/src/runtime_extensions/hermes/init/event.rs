//! Init runtime extension event handler implementation.

use crate::event::HermesEventPayload;

/// Init event
pub(crate) struct InitEvent {}

impl HermesEventPayload for InitEvent {
    fn event_name(&self) -> &'static str {
        "init"
    }

    #[allow(unreachable_code)]
    fn execute(&self, module: &mut crate::wasm::module::ModuleInstance) -> anyhow::Result<()> {
        println!("executing init event");

        let _res = module
            .instance
            .hermes_init_event()
            .call_init(&mut module.store)?;
        Ok(())
    }
}
