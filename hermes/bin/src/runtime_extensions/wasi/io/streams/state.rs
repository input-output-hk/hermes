//! Streams state.

use once_cell::sync::Lazy;

use crate::runtime_extensions::{
    bindings::wasi::io::streams::{InputStream, OutputStream},
    resource_manager::ApplicationResourceManager,
};

/// Helper super trait for `InputStream` which wraps a `std::io::Read` and
/// `std::io::Seek`.
pub(crate) trait InputStreamTrait: std::io::Read + std::io::Seek + Send + Sync {}
impl<T: std::io::Read + std::io::Seek + Send + Sync> InputStreamTrait for T {}

/// Map of app name to input streams resource holder.
pub(crate) type InputStreams = ApplicationResourceManager<InputStream, Box<dyn InputStreamTrait>>;

/// Global state to hold the input streams resources.
static INPUT_STREAMS_STATE: Lazy<InputStreams> = Lazy::new(InputStreams::new);

/// Helper super trait for `OutputStream` which wraps a `std::io::Write` and
/// `std::io::Seek`.
pub(crate) trait OutputStreamTrait: std::io::Write + std::io::Seek + Send + Sync {}
impl<T: std::io::Write + std::io::Seek + Send + Sync> OutputStreamTrait for T {}

/// Map of app name to output streams resource holder.
pub(crate) type OutputStreams =
    ApplicationResourceManager<OutputStream, Box<dyn OutputStreamTrait>>;

/// Global state to hold the input streams resources.
static OUTPUT_STREAMS_STATE: Lazy<OutputStreams> = Lazy::new(OutputStreams::new);

/// Get the input streams state.
pub(crate) fn get_input_streams_state() -> &'static InputStreams {
    &INPUT_STREAMS_STATE
}

/// Get the output streams state.
pub(crate) fn get_output_streams_state() -> &'static OutputStreams {
    &OUTPUT_STREAMS_STATE
}
