//! KV-Store host implementation for WASM runtime.

use crate::{
    runtime_extensions::bindings::hermes::kv_store::api::{Host, KvValues},
    state::HermesState,
};

impl Host for HermesState {
    /// Set a value in the local key-value store
    /// Setting None will cause the Key to be deleted from the KV store.
    fn kv_set(&mut self, _key: String, _value: Option<KvValues>) -> wasmtime::Result<()> {
        todo!()
    }

    /// Get a value from the local key-value store
    /// Returns the default if not set.
    fn kv_get_default(
        &mut self, _key: String, _default: Option<KvValues>,
    ) -> wasmtime::Result<Option<KvValues>> {
        todo!()
    }

    /// Get a value from the local key-value store
    /// Returns None if the Key does not exist in the KV Store.
    /// This is a convenience function, and is equivalent to `kv-get-default(key, none)`
    fn kv_get(&mut self, _key: String) -> wasmtime::Result<Option<KvValues>> {
        todo!()
    }

    /// Get a value, and then set it (Atomic)
    /// Setting None will cause the Key to be deleted from the KV store.
    fn kv_get_set(
        &mut self, _key: String, _value: Option<KvValues>,
    ) -> wasmtime::Result<Option<KvValues>> {
        todo!()
    }

    /// Get a value, and then add to it (Atomic)
    /// Adding to a string will concatenate the string.
    /// String concatenation will only occur up to the maximum possible size of a string
    /// value.\\
    /// Concatenation beyond the maximum size will result in truncation.
    /// Adding to a numeric will have the expected behavior (rounded to nearest if
    /// necessary).
    /// The original type does not change, so: `float64 + u64 = float64`.  `s64 + float64
    /// = s64`
    /// If the value overflows or under-flows it will saturate at the limit.
    /// This behavior allows us to decrement values by using the signed version, so
    /// `u64(10) + s64(-5) = u64(5))`
    /// If a string is added to a numeric, nothing happens.
    /// If a numeric is added to a string, it is converted to a string first, and then
    /// concatenated
    /// Note: There will be no spaces added.  So "My string" + u32(77) = "My string77"
    fn kv_add(
        &mut self, _key: String, _value: Option<KvValues>,
    ) -> wasmtime::Result<Option<KvValues>> {
        todo!()
    }

    /// Check if the Key equals a test value (exact match) and if it does, store the new
    /// value.
    /// In all cases, the current value is returned.
    /// If the types are NOT the same, the comparison will fail, even if the values are
    /// equivalent.
    /// For example: `u64(7) != s64(7)`, `float64(-1) != s64(-1)`.
    fn kv_cas(
        &mut self, _key: String, _test: Option<KvValues>, _value: Option<KvValues>,
    ) -> wasmtime::Result<Option<KvValues>> {
        todo!()
    }

    /// Subscribe to any updates made to a particular Key.
    /// After this call, this module will receive Key Update events when a key is written.
    /// It returns the current value of the Key and None if it is not set.
    fn kv_subscribe(&mut self, _key: String) -> wasmtime::Result<Option<KvValues>> {
        todo!()
    }

    /// Unsubscribe to any updates made to a particular Key.
    /// After this call, this module will no longer receive Key Update events when a key
    /// is written.
    /// It returns the current value of the Key and None if it is not set.
    fn kv_unsubscribe(&mut self, _key: String) -> wasmtime::Result<Option<KvValues>> {
        todo!()
    }
}
