//! Hermes Integration Test host implementation for WASM runtime.

use crate::{
    runtime_context::HermesRuntimeContext,
    runtime_extensions::bindings::hermes::integration_test::api::Host,
};

impl Host for HermesRuntimeContext {}
