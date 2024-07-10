//! Module provides different objects, abtractions for working with HDF5 packages.

mod compression;
#[allow(dead_code)]
mod dir;
mod path;
#[allow(dead_code, unused_imports)]
pub(crate) mod resources;

use std::fmt::Display;

/// File open and read error.
#[derive(thiserror::Error, Debug)]
pub(crate) struct FileError {
    /// File location.
    location: String,
    /// File open and read error.
    msg: Option<anyhow::Error>,
}
impl FileError {
    /// Create a new `FileError` instance from a string location.
    #[allow(dead_code)]
    fn from_string(location: String, msg: Option<anyhow::Error>) -> Self {
        Self { location, msg }
    }

    /// Create a new `FileError` instance from a path location.
    fn from_path<P: AsRef<std::path::Path>>(path: P, msg: Option<anyhow::Error>) -> Self {
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
