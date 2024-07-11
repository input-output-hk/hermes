//! Module provides different objects, abstractions for working with HDF5 packages.

mod compression;
mod dir;
mod file;
mod path;
pub(crate) mod resources;

pub(crate) use dir::Dir;
pub(crate) use file::File;
pub(crate) use path::Path;
