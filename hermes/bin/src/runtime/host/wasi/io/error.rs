//! WASI IO Error
//!
#![allow(unused_variables)]

use crate::runtime::extensions::{
    wasi::io::error::{Error, Host, HostError},
    HermesState, NewState,
};

/// WASI State
pub(crate) struct State {}

impl NewState for State {
    fn new(_ctx: &crate::wasm::context::Context) -> Self {
        Self {}
    }
}

impl HostError for HermesState {
    #[doc = " Returns a string that is suitable to assist humans in debugging"]
    #[doc = " this error."]
    #[doc = " "]
    #[doc = " WARNING: The returned string should not be consumed mechanically!"]
    #[doc = " It may change across platforms, hosts, or other implementation"]
    #[doc = " details. Parsing this string is a major platform-compatibility"]
    #[doc = " hazard."]
    fn to_debug_string(
        &mut self, self_: wasmtime::component::Resource<Error>,
    ) -> wasmtime::Result<String> {
        todo!()
    }

    fn drop(&mut self, rep: wasmtime::component::Resource<Error>) -> wasmtime::Result<()> {
        todo!()
    }
}

impl Host for HermesState {}
