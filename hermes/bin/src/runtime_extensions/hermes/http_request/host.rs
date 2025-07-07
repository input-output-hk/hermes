//!  Cardano Blockchain host implementation for WASM runtime.

use crate::{
    runtime_context::HermesRuntimeContext,
    runtime_extensions::{
        bindings::hermes::http_request::api::{Host, Payload},
        hermes::http_request::{tokio_runtime_task::{parse_payload, ParsedPayload}, STATE},
    },
};

impl Host for HermesRuntimeContext {
    fn send(&mut self, payload: Payload) -> wasmtime::Result<bool> {
        STATE.tokio_rt_handle.send(payload).unwrap();

        Ok(true)
    }
}
