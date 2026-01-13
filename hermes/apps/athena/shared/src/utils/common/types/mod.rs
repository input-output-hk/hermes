//! Common types
//!
//! These should be simple types, not objects.
//! For example, types derived from strings or integers and vectors of simple types only.
//!
//! Objects are objects, and not types.
//!
//! Simple types can be enums, if the intended underlying type is simple, such as a string
//! or integer.

pub(crate) mod array_types;
pub mod cardano;
pub(crate) mod generic;
pub(crate) mod headers;
pub(crate) mod string_types;

/// Wrapper for `cid::Cid` to provide data validation during selection from database.
#[derive(Clone, Debug, Default)]
pub struct Cid(pub cid::Cid);

impl Cid {
    /// Returns CID bytes.
    #[must_use]
    pub fn to_bytes(&self) -> Vec<u8> {
        self.0.to_bytes()
    }

    /// Creates from raw bytes (defaults to empty CID on error).
    #[must_use]
    pub fn from_bytes(bytes: &[u8]) -> Self {
        Self(cid::Cid::try_from(bytes).unwrap_or_default())
    }

    /// Creates from raw bytes (defaults to empty CID on error).
    ///
    /// # Errors
    ///
    /// Returns an error if fails to create CID from bytes.
    pub fn try_from_bytes(bytes: &[u8]) -> anyhow::Result<Self> {
        Ok(Self(cid::Cid::try_from(bytes)?))
    }

    /// Access inner CID.
    #[must_use]
    pub fn inner(&self) -> cid::Cid {
        self.0
    }
}
