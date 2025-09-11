//! KV-Store runtime extension event handler implementation.

use crate::{
    event::HermesEventPayload,
    runtime_extensions::bindings::{hermes::kv_store::api::KvValues, unchecked_exports},
};

unchecked_exports::define! {
    /// Extends [`wasmtime::component::Instance`] with guest functions for KV storage.
    trait ComponentInstanceExt {
        #[wit("hermes:kv-store/event", "kv-update")]
        fn hermes_kv_store_event_kv_update<'p>(key: &'p str, value: &'p KvValues);
    }
}

/// KV update event
#[allow(dead_code)]
struct KVUpdateEvent {
    /// Key.
    key: String,
    /// Value.
    value: KvValues,
}

impl HermesEventPayload for KVUpdateEvent {
    fn event_name(&self) -> &'static str {
        "kv-update"
    }

    fn execute(
        &self,
        module: &mut crate::wasm::module::ModuleInstance,
    ) -> anyhow::Result<()> {
        module.instance.hermes_kv_store_event_kv_update(
            &mut module.store,
            &self.key,
            &self.value,
        )?;
        Ok(())
    }
}
