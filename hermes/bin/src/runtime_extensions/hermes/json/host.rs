//! JSON host implementation for WASM runtime.

use crate::{runtime_extensions::bindings::hermes::json::api::Host, state::HermesState};

impl Host for HermesState {}
