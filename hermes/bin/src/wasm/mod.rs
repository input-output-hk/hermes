//! WASM related structures and functions.
//! All implementation based on [wasmtime](https://crates.io/crates/wasmtime) crate dependency.

mod engine;
mod module;

/// WASM module errors.
#[derive(thiserror::Error, Debug)]
enum Error {
    /// Exports mismatch
    #[error("Exports mismatch")]
    ExportsMismatch,

    /// Imports mismatch
    #[error("Imports mismatch")]
    ImportsMismatch,

    /// Export module entity is not a function
    #[error("Export module entity is not a function, name: {0}")]
    ExportNotAFunc(String),

    /// Import module entity is not a function
    #[error("Import module entity is not a function, module: {0}, name: {1}")]
    ImportNotAFunc(String, String),

    /// Internal wasmtime errors
    #[error(transparent)]
    Wasm(#[from] wasmtime::Error),
}
