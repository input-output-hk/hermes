//! Hermes Init host implementation for WASM runtime.
use std::process::ExitCode;

use crate::{
    event, runtime_context::HermesRuntimeContext,
    runtime_extensions::bindings::hermes::init::api::Host,
};

impl Host for HermesRuntimeContext {
    /// Perform Hermes event queue shut down.
    fn done(&mut self, status: Result<(), ()>) -> wasmtime::Result<()> {
        if status.is_ok() {
            event::queue::shutdown(ExitCode::SUCCESS)
        } else {
            event::queue::shutdown(ExitCode::FAILURE)
        }
    }

    /// Perform Hermes event queue shutdown with a custom status code.
    fn done_with_code(&mut self, status_code: u8) -> wasmtime::Result<()> {
        event::queue::shutdown(ExitCode::from(status_code))
    }
}
