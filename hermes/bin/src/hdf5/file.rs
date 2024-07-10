//! A Hermes HDF5 file abtraction over the HDF5 dataset object.

use std::io::Read;

/// Hermes HDF5 file object, wrapper of `hdf5::Dataset`
#[derive(Clone, Debug)]
pub(crate) struct File(hdf5::Dataset);

impl File {
    /// Create new `File`.
    pub(crate) fn new(dataset: hdf5::Dataset) -> Self {
        Self(dataset)
    }

    /// Return `File` name.
    pub(crate) fn name(&self) -> String {
        self.0.name().to_string()
    }

    /// Return `File` reader.
    pub(crate) fn reader(&self) -> anyhow::Result<impl Read> {
        Ok(self.0.as_byte_reader()?)
    }
}
