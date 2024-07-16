//! Hermes virtual file system.

mod bootstrap;

pub(crate) use bootstrap::VfsBootstraper;

use crate::hdf5::{self as hermes_hdf5};

/// Hermes virtual file system type.
pub(crate) struct Vfs {
    /// HDFR5 root directory of the virtual file system.
    #[allow(dead_code)]
    root: hermes_hdf5::Dir,
}
