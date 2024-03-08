//! Hermes `wasmtime::component::bindgen` generated code
//!
//! *Note*
//! Inspect the generated code with:
//! `cargo expand -p hermes --lib runtime_extensions::bindings`
//! or with:
//! `earthly +bindings-expand`

#![allow(clippy::indexing_slicing)]

use wasmtime::component::bindgen;

bindgen!({
    world: "hermes",
    path: "../../wasm/wasi/wit",
});
