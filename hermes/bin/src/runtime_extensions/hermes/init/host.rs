//! Hermes Init host implementation for WASM runtime.
use crate::{
    event, runtime_context::HermesRuntimeContext,
    runtime_extensions::bindings::hermes::init::api::Host,
};

impl Host for HermesRuntimeContext {
    /// Perform Hermes event queue shut down.
    fn done(&mut self, exit_code: i64) -> wasmtime::Result<()> {
        event::queue::shutdown(exit_code)
    }
}
