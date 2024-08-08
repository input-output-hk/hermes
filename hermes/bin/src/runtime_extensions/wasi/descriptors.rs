//! Hermes WASI filesystem descriptors.

/// Identifier for the null file descriptor which discards all input/output.
pub(crate) const NUL_REP: u32 = 0;
/// Identifier for the STDOUT file descriptor.
/// NOTE: This is temporary and should be removed once we redirect STDOUT to the logging
/// API.
pub(crate) const STDOUT_REP: u32 = 1;
/// Identifier for the STDERR file descriptor.
/// NOTE: This is temporary and should be removed once we redirect STDERR to the logging
/// API.
pub(crate) const STDERR_REP: u32 = 2;

/// Represents an open file or directory.
#[derive(Clone, Debug)]
pub enum Descriptor {
    /// File descriptor.
    File(crate::hdf5::File),
    /// Directory descriptor.
    Dir(crate::hdf5::Dir),
}

/// Represents an open output stream.
#[derive(Clone, Debug, Default)]
pub struct Stream {
    /// Stream position in the file.
    at: u64,
}

impl Stream {
    /// Creates a new output stream associated with the given file descriptor at
    /// the given offset.
    pub(crate) fn new(offset: u64) -> Self {
        Self { at: offset }
    }

    /// Advances the stream offset.
    pub(crate) fn advance(&mut self, len: u64) {
        self.at += len;
    }

    /// Returns the stream offset.
    pub(crate) fn at(&self) -> u64 {
        self.at
    }
}
