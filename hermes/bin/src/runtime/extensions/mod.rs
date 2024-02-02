//! Runtime modules - extensions
//!
//! *Note*
//! Inspect the generated code with:
//! ```
//! cargo expand --bin hermes runtime::extensions
//! ```
#![allow(clippy::indexing_slicing)]

use wasmtime::component::bindgen;

bindgen!({
    world: "hermes",
    path: "../../wasm/wasi/wit",
});

//mod hermes;
//mod wasi;
