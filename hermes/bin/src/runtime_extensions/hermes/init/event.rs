//! Init runtime extension event handler implementation.

use crate::{event::HermesEventPayload, runtime_extensions::bindings::unchecked_exports};

unchecked_exports::define! {
    /// Extends [`wasmtime::component::Instance`] with guest functions for init.
    trait ComponentInstanceExt {
        #[wit("hermes:init/event", "init")]
        fn hermes_init_event_init() -> bool;
    }
}

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
        let _res = module.instance.hermes_init_event_init(&mut module.store)?;
        Ok(())
    }
}
