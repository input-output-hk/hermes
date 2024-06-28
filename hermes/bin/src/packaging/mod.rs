//! Hermes packaging.

#[allow(dead_code)]
pub(crate) mod app;
pub(crate) mod hash;
pub(crate) mod package;
mod resources;
mod schema_validation;
pub(crate) mod sign;
pub(crate) mod wasm_module;

use std::{fmt::Display, path::Path};

/// File open and read error.
#[derive(thiserror::Error, Debug)]
struct FileError {
    /// File location.
    location: String,
    /// File open and read error.
    msg: Option<anyhow::Error>,
}
impl FileError {
    /// Create a new `FileError` instance from a string location.
    fn from_string(location: String, msg: Option<anyhow::Error>) -> Self {
        Self { location, msg }
    }

    /// Create a new `FileError` instance from a path location.
    fn from_path<P: AsRef<Path>>(path: P, msg: Option<anyhow::Error>) -> Self {
        Self {
            location: path.as_ref().display().to_string(),
            msg,
        }
    }
}
impl Display for FileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let msg = format!("Cannot open or read file at {0}", self.location);
        let err = self
            .msg
            .as_ref()
            .map(|msg| format!(":\n{msg}"))
            .unwrap_or_default();
        writeln!(f, "{msg}{err}",)
    }
}
