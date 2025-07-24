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

/*

q := [Continue(ev1), Continue(ev2), Continue(ev3), Continue(ev4)]

# ... one of the currently executed modules
> shutdown(1)

q := [.., Continue(ev3), Continue(ev4), Break(1)]

execute ev1
execute ev2
execute ev3
execute ev4
execute Break(1) -> queue is dead

*/