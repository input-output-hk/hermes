//! Binary host implementation for WASM runtime.

use crate::{
    runtime_extensions::bindings::hermes::binary::api::Host, runtime_context::HermesRuntimeContext,
};

impl Host for HermesRuntimeContext {}
