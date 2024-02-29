//! JSON host implementation for WASM runtime.

use crate::{
    runtime_context::HermesRuntimeContext, runtime_extensions::bindings::hermes::json::api::Host,
};

impl Host for HermesRuntimeContext {}
