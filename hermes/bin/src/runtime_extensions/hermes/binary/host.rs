//! Binary host implementation for WASM runtime.

use crate::{runtime_extensions::bindings::hermes::binary::api::Host, state::HermesState};

impl Host for HermesState {}
