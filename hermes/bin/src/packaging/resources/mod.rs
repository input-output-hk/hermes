//! Resources module functionality.

pub(crate) mod bytes;
mod fs;
mod hdf5;
mod manifest;
mod uri;

use std::{
    fmt::{Debug, Display},
    io::Read,
};

pub(crate) use bytes::BytesResource;
pub(crate) use fs::FsResource;
pub(crate) use hdf5::Hdf5Resource;
pub(crate) use manifest::ManifestResource;

/// Resource trait definition.
pub(crate) trait ResourceTrait: Display {
    /// Get resource name (e.g. file name or dir name).
    fn name(&self) -> anyhow::Result<String>;

    /// Check if resource is a directory.
    fn is_dir(&self) -> bool;

    /// Check if resource is a file.
    fn is_file(&self) -> bool;

    /// Get data reader for the resource.
    fn get_reader(&self) -> anyhow::Result<impl Read + Debug>;

    /// Get directory content.
    fn get_directory_content(&self) -> anyhow::Result<Vec<Self>>
    where Self: Sized;
}
