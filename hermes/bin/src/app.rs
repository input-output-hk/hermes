//! Hermes app implementation.

use std::sync::Arc;

use crate::{
    event_queue::HermesEventQueueOut, runtime_extensions::state::State, wasm::module::Module,
};

/// Hermes app
pub(crate) struct HermesApp {
    /// WASM modules
    wasm_modules: Vec<Module>,
}

impl HermesApp {
    /// Create a new Hermes app
    pub(crate) fn new(app_name: &str, module_bytes: Vec<Vec<u8>>) -> anyhow::Result<Self> {
        let mut wasm_modules = Vec::with_capacity(module_bytes.len());
        for module_bytes in module_bytes {
            wasm_modules.push(Module::new(app_name.to_string(), &module_bytes)?);
        }
        Ok(Self { wasm_modules })
    }

    /// Executes Hermes events from the event queue channel.
    ///
    /// # Note:
    /// This is a blocking call until event queue channel is open.
    pub(crate) fn event_execution_loop(
        &mut self, event_queue_out: HermesEventQueueOut, state: &Arc<State>,
    ) -> anyhow::Result<()> {
        for event in event_queue_out {
            for module in &mut self.wasm_modules {
                module.execute_event(event.as_ref(), state.clone())?;
            }
        }
        Ok(())
    }
}
