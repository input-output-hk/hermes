//! Select from the database.

pub(crate) mod cat_id;
pub(crate) mod stake_addr;

/// Enum to track which table the registration came from.
#[derive(Debug, Clone)]
pub(crate) enum TableSource {
    /// Persistent data.
    Persistent,
    /// Volatile data.
    Volatile,
}
