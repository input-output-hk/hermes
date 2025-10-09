//! Hermes HTTP Gateway host implementation for WASM runtime.

use crate::{
    runtime_context::HermesRuntimeContext,
    runtime_extensions::bindings::hermes::http_gateway::api::Host,
};

impl Host for HermesRuntimeContext {}
