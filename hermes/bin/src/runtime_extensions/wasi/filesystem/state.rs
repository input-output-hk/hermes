//! Filesystem state.

use once_cell::sync::Lazy;

use crate::runtime_extensions::{
    bindings::wasi::filesystem, resource_manager::ApplicationResourceManager,
};
/// Map of app name to descriptors.
pub(crate) type Descriptors = ApplicationResourceManager<filesystem::types::Descriptor, Descriptor>;

/// Represents an open file or directory.
#[derive(Clone, Debug)]
pub(crate) enum Descriptor {
    /// File descriptor.
    File(crate::hdf5::File),
    /// Directory descriptor.
    Dir(crate::hdf5::Dir),
}

/// Global state to hold the descriptors resources.
static DESCRIPTORS_STATE: Lazy<Descriptors> = Lazy::new(Descriptors::new);

/// Get the filesystem state.
pub(super) fn get_state() -> &'static Descriptors {
    &DESCRIPTORS_STATE
}
