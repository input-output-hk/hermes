//! WASM related structures and functions.
//! All implementation based on [wasmtime](https://crates.io/crates/wasmtime) crate dependecy.

mod engine;
mod module;

/// WASM module errors.
#[derive(thiserror::Error, Debug)]
enum Error {
    /// Exports mistmatch
    #[error("Exports mistmatch")]
    ExportsMismatch,

    /// Imports mistmatch
    #[error("Imports mistmatch")]
    ImportsMismatch,

    /// Export entity is not a function
    #[error("Export entity is not a function, name: {0}")]
    ExportNotAFunc(String),

    /// Import entity is not a function
    #[error("Import entity is not a function, module: {0}, name: {1}")]
    ImportNotAFunc(String, String),

    /// Internal wasmtime errors
    #[error(transparent)]
    Wasm(#[from] wasmtime::Error),
}
