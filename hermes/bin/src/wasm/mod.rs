mod engine;
mod module;

#[derive(thiserror::Error, Debug)]
enum Error {
    #[error("Exports mistmatch")]
    ExportsMismatch,

    #[error("Imports mistmatch")]
    ImportsMismatch,

    #[error("Export not a function, name: {0}")]
    ExportNotAFunc(String),

    #[error("Import not a function, module: {0}, name: {1}")]
    ImportNotAFunc(String, String),

    #[error(transparent)]
    Wasm(#[from] wasmtime::Error),
}
