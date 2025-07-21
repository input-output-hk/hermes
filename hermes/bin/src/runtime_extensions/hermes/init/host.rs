//! Hermes Init host implementation for WASM runtime.
use std::process::ExitCode;

use crate::{
    event, runtime_context::HermesRuntimeContext,
    runtime_extensions::bindings::hermes::init::api::Host,
};

impl Host for HermesRuntimeContext {
    /// Perform Hermes event queue shutdown.
    fn done(&mut self, status_code: u8) -> wasmtime::Result<()> {
        event::queue::shutdown(ExitCode::from(status_code))
    }
}
