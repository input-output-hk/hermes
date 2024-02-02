//! Runtime modules - extensions - Hermes - Binary extensions
//!
//! *Note*
//! Inspect the generated code with:
//! ```
//! cargo expand --bin hermes runtime::extensions::hermes::binary
//! ```

use wasmtime::component::bindgen;

bindgen!({
    world: "hermes:binary/all",
    path: "../../wasm/wasi/wit",
});
