//! Binary host implementation for WASM runtime.

use crate::{
    runtime_extensions::bindings::hermes::binary::api::Host, runtime_state::HermesRuntimeState,
};

impl Host for HermesRuntimeState {}
