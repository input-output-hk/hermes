//! Select from the database.

pub(crate) mod cat_id;

/// Enum to track which table the registration came from.
#[derive(Debug, Clone)]
pub enum TableSource {
    /// Persistent data.
    Persistent,
    /// Volatile data.
    Volatile,
}
