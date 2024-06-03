//! KV-Store runtime extension event handler implementation.

use std::sync::mpsc::Sender;

use crate::{
    event::HermesEventPayload, runtime_extensions::bindings::hermes::kv_store::api::KvValues,
};

/// KV update event
struct KVUpdateEvent {
    /// Key.
    key: String,
    /// Value.
    value: KvValues,
}

impl HermesEventPayload for KVUpdateEvent {
    fn event_name(&self) -> &str {
        "kv-update"
    }

    fn execute(&self, module: &mut crate::wasm::module::ModuleInstance) -> anyhow::Result<()> {
        module.instance.hermes_kv_store_event().call_kv_update(
            &mut module.store,
            &self.key,
            &self.value,
        )?;
        Ok(())
    }
}

/// KV get event
pub struct KVGet {
    pub(crate) event: String,
    pub(crate) sender: Sender<String>,
}

impl HermesEventPayload for KVGet {
    fn event_name(&self) -> &str {
        "kv-get"
    }

    fn execute(&self, module: &mut crate::wasm::module::ModuleInstance) -> anyhow::Result<()> {
        let value = module
            .instance
            .hermes_kv_store_event()
            .call_kv_get(&mut module.store, &self.event)?;

        self.sender.send(value.to_string()).unwrap();

        Ok(())
    }
}
