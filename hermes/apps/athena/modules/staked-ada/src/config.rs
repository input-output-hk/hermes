//! Configuration constants for the staked-ada module.

/// Regex pattern for matching stake address routes.
/// Matches paths like: /api/gateway/v1/cardano/assets/stake1...
pub const STAKE_ROUTE_PATTERN: &str = r"^/api/gateway/v1/cardano/assets/(stake1[a-z0-9]{53})$";

/// Batch size for database operations.
pub const DB_BATCH_SIZE: usize = 100;

/// Error messages.
#[allow(clippy::missing_docs_in_private_items)]
pub mod messages {
    pub const STAKE_ADDRESS_NOT_FOUND: &str = "Stake address not found";
    pub const INTERNAL_SERVER_ERROR: &str = "Internal server error";
    pub const SERVICE_UNAVAILABLE: &str = "Service unavailable";
    pub const NOT_FOUND: &str = "Not found";
    pub const UNKNOWN_ERROR: &str = "Unknown error";
    pub const SERIALIZATION_FAILED: &str = "Serialization failed";
    pub const PAGE_NOT_FOUND: &str = "404 - Page Not Found";
    pub const BAD_REQUEST: &str = "400 - Bad Request";
}
