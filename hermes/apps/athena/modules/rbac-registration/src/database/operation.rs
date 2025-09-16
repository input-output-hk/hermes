//! Database operation.

use strum::Display;

/// Database operations variation.
#[derive(Display)]
pub(crate) enum Operation {
    /// Insert operation.
    Insert,
    /// Delete operation.
    Delete,
    /// Select operation.
    SELECT,
    /// Create operation.
    CREATE,
}
