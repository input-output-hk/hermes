//! Host - KV-Store implementations
//!
#![allow(unused_variables)]

use crate::runtime::extensions::{
    hermes::kv_store::api::{Host, KvValues},
    HermesState, NewState,
};

/// State
pub(crate) struct State {}

impl NewState for State {
    fn new(_ctx: &crate::wasm::context::Context) -> Self {
        State {}
    }
}

impl Host for HermesState {
    #[doc = " Set a value in the local key-value store"]
    #[doc = " Setting None will cause the Key to be deleted from the KV store."]
    fn kv_set(&mut self, key: String, value: Option<KvValues>) -> wasmtime::Result<()> {
        todo!()
    }

    #[doc = " Get a value from the local key-value store"]
    #[doc = " Returns the default if not set."]
    fn kv_get_default(
        &mut self, key: String, default: Option<KvValues>,
    ) -> wasmtime::Result<Option<KvValues>> {
        todo!()
    }

    #[doc = " Get a value from the local key-value store"]
    #[doc = " Returns None if the Key does not exist in the KV Store."]
    #[doc = " This is a convenience function, and is equivalent to `kv-get-default(key, none)`"]
    fn kv_get(&mut self, key: String) -> wasmtime::Result<Option<KvValues>> {
        todo!()
    }

    #[doc = " Get a value, and then set it (Atomic)"]
    #[doc = " Setting None will cause the Key to be deleted from the KV store."]
    fn kv_get_set(
        &mut self, key: String, value: Option<KvValues>,
    ) -> wasmtime::Result<Option<KvValues>> {
        todo!()
    }

    #[doc = " Get a value, and then add to it (Atomic)"]
    #[doc = " Adding to a string will concatenate the string."]
    #[doc = " String concatenation will only occur up to the maximum possible size of a string value.\\\\"]
    #[doc = " Concatenation beyond the maximum size will result in truncation."]
    #[doc = " Adding to a numeric will have the expected behavior (rounded to nearest if necessary)."]
    #[doc = " The original type does not change, so: `float64 + u64 = float64`.  `s64 + float64 = s64`"]
    #[doc = " If the value overflows or under-flows it will saturate at the limit."]
    #[doc = " This behavior allows us to decrement values by using the signed version, so `u64(10) + s64(-5) = u64(5))`"]
    #[doc = " If a string is added to a numeric, nothing happens."]
    #[doc = " If a numeric is added to a string, it is converted to a string first, and then concatenated"]
    #[doc = " Note: There will be no spaces added.  So \"My string\" + u32(77) = \"My string77\""]
    fn kv_add(
        &mut self, key: String, value: Option<KvValues>,
    ) -> wasmtime::Result<Option<KvValues>> {
        todo!()
    }

    #[doc = " Check if the Key equals a test value (exact match) and if it does, store the new value."]
    #[doc = " In all cases, the current value is returned."]
    #[doc = " If the types are NOT the same, the comparison will fail, even if the values are equivalent."]
    #[doc = " For example: `u64(7) != s64(7)`, `float64(-1) != s64(-1)`."]
    fn kv_cas(
        &mut self, key: String, test: Option<KvValues>, value: Option<KvValues>,
    ) -> wasmtime::Result<Option<KvValues>> {
        todo!()
    }

    #[doc = " Subscribe to any updates made to a particular Key."]
    #[doc = " After this call, this module will receive Key Update events when a key is written."]
    #[doc = " It returns the current value of the Key and None if it is not set."]
    fn kv_subscribe(&mut self, key: String) -> wasmtime::Result<Option<KvValues>> {
        todo!()
    }

    #[doc = " Unsubscribe to any updates made to a particular Key."]
    #[doc = " After this call, this module will no longer receive Key Update events when a key is written."]
    #[doc = " It returns the current value of the Key and None if it is not set."]
    fn kv_unsubscribe(&mut self, key: String) -> wasmtime::Result<Option<KvValues>> {
        todo!()
    }
}
