//! Hermes WASI filesystem descriptors.

/// Represents an open file or directory.
#[derive(Clone, Debug)]
pub enum Descriptor {
    /// File descriptor.
    File(crate::hdf5::File),
    /// Directory descriptor.
    Dir(crate::hdf5::Dir),
}
