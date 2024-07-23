//! Hermes virtual file system.

mod bootstrap;

pub(crate) use bootstrap::{Hdf5Mount, Hdf5MountToLib, VfsBootstrapper};

use crate::hdf5::{self as hermes_hdf5};

/// Hermes virtual file system type.
pub(crate) struct Vfs {
    /// HDF5 root directory of the virtual file system.
    #[allow(dead_code)]
    root: hermes_hdf5::Dir,
}
