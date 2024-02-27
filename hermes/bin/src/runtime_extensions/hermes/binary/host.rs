//! Binary host implementation for WASM runtime.

use crate::{runtime_extensions::bindings::hermes::binary::api::Host, state::HermesRuntimeState};

impl Host for HermesRuntimeState {}
