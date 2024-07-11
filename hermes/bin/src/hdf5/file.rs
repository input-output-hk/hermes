//! A Hermes HDF5 file abstraction over the HDF5 dataset object.

use std::io::Read;

use super::Path;

/// Hermes HDF5 file object, wrapper of `hdf5::Dataset`
#[derive(Clone, Debug)]
pub(crate) struct File(hdf5::Dataset);

impl File {
    /// Create new `File`.
    pub(crate) fn new(dataset: hdf5::Dataset) -> Self {
        Self(dataset)
    }

    /// Return file `Path`.
    pub(crate) fn path(&self) -> Path {
        Path::from_str(&self.0.name())
    }

    /// Return file reader.
    pub(crate) fn reader(&self) -> anyhow::Result<impl Read> {
        Ok(self.0.as_byte_reader()?)
    }
}
