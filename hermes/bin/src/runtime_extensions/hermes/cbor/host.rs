//! CBOR host implementation for WASM runtime.

use crate::{runtime_extensions::bindings::hermes::cbor::api::Host, runtime_state::HermesRuntimeState};

impl Host for HermesRuntimeState {}
