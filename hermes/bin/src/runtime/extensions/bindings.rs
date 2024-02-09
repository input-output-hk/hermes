//! Hermes `wasmtime::component::bindgen` generated code
//!
//! *Note*
//! Inspect the generated code with:
//! `cargo expand --bin hermes runtime::extensions::bindings`

#![allow(clippy::indexing_slicing)]

use wasmtime::component::bindgen;

bindgen!({
    world: "hermes",
    path: "../../wasm/wasi/wit",
});
