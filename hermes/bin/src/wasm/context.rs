//! WASM host context implementation.

/// A WASM host context structure, which is intended to be passed to the `wasmtime::Store`
/// during the WASM state initialization process.
#[derive(Clone)]
pub(crate) struct Context;
