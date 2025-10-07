//! Implement type wrapper for boolean type

use derive_more::{From, Into};

/// Boolean flag
#[derive(Debug, Clone, From, Into)]
pub(crate) struct BooleanFlag(bool);

impl Default for BooleanFlag {
    /// Explicit default implementation of `Flag`.
    fn default() -> Self {
        Self(true)
    }
}
