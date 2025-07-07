//!  Cardano Blockchain host implementation for WASM runtime.

use crate::{
    runtime_context::HermesRuntimeContext,
    runtime_extensions::bindings::hermes::http_request::api::{Host, Payload as ApiPayload},
};

impl Host for HermesRuntimeContext {
    fn send(&mut self, payload: ApiPayload) -> wasmtime::Result<bool> {
        tracing::error!("Sending payload: {payload:?}");

        // Convert from ApiPayload to internal Payload
        let internal_payload = super::Payload {
            host_uri: payload.host_uri,
            port: payload.port,
            body: payload.body,
            request_id: payload.request_id,
        };

        // Use the send function from the module instead of accessing the handle directly
        match super::send(internal_payload) {
            Ok(success) => Ok(success),
            Err(e) => {
                tracing::error!("Failed to send HTTP request: {:?}", e);
                Ok(false)
            }
        }
    }
}