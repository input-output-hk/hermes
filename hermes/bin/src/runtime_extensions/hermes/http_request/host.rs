//!  Cardano Blockchain host implementation for WASM runtime.

use crate::{
    runtime_context::HermesRuntimeContext,
    runtime_extensions::bindings::hermes::http_request::api::{Host, Payload},
};

impl Host for HermesRuntimeContext {
    fn send(&mut self, p: Payload) -> wasmtime::Result<bool> {
        todo!()
    }
}
