//! Streams state.

use once_cell::sync::Lazy;

use crate::runtime_extensions::{
    bindings::wasi::io::streams::InputStream, resource_manager::ApplicationResourceManager,
};

/// Helper super trait for `InputStream` which wraps a `std::io::Read` and
/// `std::io::Seek`.
pub(crate) trait InputStreamTrait: std::io::Read + std::io::Seek + Send + Sync {}
impl<T: std::io::Read + std::io::Seek + Send + Sync> InputStreamTrait for T {}

/// Map of app name to input streams resource holder.
pub(crate) type InputStreams = ApplicationResourceManager<InputStream, Box<dyn InputStreamTrait>>;

/// Global state to hold the input streams resources.
static INPUT_STREAMS_STATE: Lazy<InputStreams> = Lazy::new(InputStreams::new);

/// Get the input streams state.
pub(crate) fn get_intput_streams_state() -> &'static InputStreams {
    &INPUT_STREAMS_STATE
}
