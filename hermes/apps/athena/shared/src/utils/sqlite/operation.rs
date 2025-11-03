//! Database operation.

use strum::Display;

/// Database operations variation.
#[derive(Display, Clone, Copy)]
pub enum Operation {
    /// Insert operation.
    Insert,
    /// Delete operation.
    Delete,
    /// Select operation.
    Select,
    /// Create operation.
    Create,
}
