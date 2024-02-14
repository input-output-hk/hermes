//! IO Error host implementation for WASM runtime.

use crate::{
    runtime_extensions::bindings::wasi::io::error::{Error, Host, HostError},
    state::HermesState,
};

impl HostError for HermesState {
    /// Returns a string that is suitable to assist humans in debugging
    /// this error.
    ///
    /// WARNING: The returned string should not be consumed mechanically!
    /// It may change across platforms, hosts, or other implementation
    /// details. Parsing this string is a major platform-compatibility
    /// hazard.
    fn to_debug_string(
        &mut self, _rep: wasmtime::component::Resource<Error>,
    ) -> wasmtime::Result<String> {
        todo!()
    }

    fn drop(&mut self, _rep: wasmtime::component::Resource<Error>) -> wasmtime::Result<()> {
        todo!()
    }
}

impl Host for HermesState {}
