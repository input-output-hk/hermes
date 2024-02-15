//! KV-Store runtime extension event handler implementation.

use crate::{
    event_queue::event::HermesEventPayload,
    runtime_extensions::bindings::hermes::kv_store::api::KvValues,
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
