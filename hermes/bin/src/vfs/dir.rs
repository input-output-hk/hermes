//! Hermes virtual file system directory object implementation.

#![allow(dead_code)]

use crate::hdf5 as hermes_hdf5;

/// Hermes virtual file system directory struct.
pub(crate) struct Dir {
    /// HDF5 directory.
    dir: hermes_hdf5::Dir,
}

impl Dir {
    /// Create a new `Dir` instance.
    pub(crate) fn new(dir: hermes_hdf5::Dir) -> Self {
        Self { dir }
    }

    /// Returns the name of the directory.
    pub(crate) fn name(&self) -> String {
        self.dir.name()
    }
}
