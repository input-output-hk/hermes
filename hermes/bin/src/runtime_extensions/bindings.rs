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
    with: {
        "wasi:filesystem/types/descriptor": super::wasi::descriptors::Descriptor,
        "wasi:io/streams/input-stream": super::wasi::descriptors::Stream,
        "wasi:io/streams/output-stream": super::wasi::descriptors::Stream,
    }
});
