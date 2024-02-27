//! CBOR host implementation for WASM runtime.

use crate::{runtime_extensions::bindings::hermes::cbor::api::Host, state::HermesRuntimeState};

impl Host for HermesRuntimeState {}
