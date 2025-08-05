//!  Http Request host implementation for WASM runtime.

use crate::{
    runtime_context::HermesRuntimeContext,
    runtime_extensions::{
        bindings::hermes::http_request::api::{ErrorCode, Host, Payload},
        hermes::http_request::STATE,
    },
};

impl Host for HermesRuntimeContext {
    fn send(
        &mut self,
        payload: Payload,
    ) -> wasmtime::Result<Result<(), ErrorCode>> {
        STATE.tokio_rt_handle.send(payload)?;

        Ok(Ok(()))
    }
}
