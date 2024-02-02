//! Runtime modules - extensions

//! Runtime modules - extensions - Hermes - Binary extensions
//!
//! *Note*
//! Inspect the generated code with:
//! ```
//! cargo expand --bin hermes runtime::extensions::hermes::binary
//! ```
#![allow(clippy::indexing_slicing)]

use wasmtime::component::bindgen;

bindgen!({
    world: "hermes",
    path: "../../wasm/wasi/wit",
});

//mod hermes;
//mod wasi;
