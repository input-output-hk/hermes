//! Hermes `wasmtime::component::bindgen` generated code
//!
//! *Note*
//! Inspect the generated code with:
//! `cargo expand -p hermes --lib runtime_extensions::bindings`
//! or with:
//! `earthly +bindings-expand`

use wasmtime::component::bindgen;

bindgen!({
    path: "../../wasm/wasi/wit",
    trappable_imports: true,
});
