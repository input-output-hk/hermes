//! Init runtime extension event handler implementation.

use crate::{
    event::HermesEventPayload,
    runtime_extensions::bindings::unchecked_exports::ComponentInstanceExt as _,
};

/// Init event
pub(crate) struct InitEvent {}

impl HermesEventPayload for InitEvent {
    fn event_name(&self) -> &'static str {
        "init"
    }

    fn execute(
        &self,
        module: &mut crate::wasm::module::ModuleInstance,
    ) -> anyhow::Result<()> {
        let (_res,): (bool,) = module
            .instance
            .hermes_init_event_init(&mut module.store)?
            .call(&mut module.store, ())?;
        Ok(())
    }
}
