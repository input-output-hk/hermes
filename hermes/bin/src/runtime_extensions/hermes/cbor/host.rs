//! CBOR host implementation for WASM runtime.

use crate::{runtime_extensions::bindings::hermes::cbor::api::Host, state::HermesState};

impl Host for HermesState {}
