//! Error types for the staked-ada module.

use thiserror::Error;

/// Errors that can occur in the staked-ada module.
#[derive(Debug, Error)]
pub enum StakedAdaError {
    /// Invalid stake address format.
    #[error("Invalid stake address format: {address}")]
    InvalidStakeAddress { address: String },

    /// Database connection or query error.
    #[error("Database error: {source}")]
    Database { source: anyhow::Error },

    /// Network mismatch between provided and expected.
    #[error("Network mismatch: expected {expected:?}, got {actual:?}")]
    NetworkMismatch {
        expected: shared::utils::common::objects::cardano::network::Network,
        actual: shared::utils::common::objects::cardano::network::Network,
    },

    /// Stake address not found in database.
    #[error("Stake address not found: {address}")]
    StakeAddressNotFound { address: String },

    /// JSON serialization/deserialization error.
    #[error("Serialization error: {source}")]
    Serialization { source: serde_json::Error },

    /// Invalid path format for route matching.
    #[error("Invalid path format: {path}")]
    InvalidPath { path: String },

    /// General validation error.
    #[error("Validation error: {message}")]
    Validation { message: String },

    /// Internal server error.
    #[error("Internal server error: {message}")]
    Internal { message: String },
}

/// Result type alias for the staked-ada module.
pub type Result<T> = std::result::Result<T, StakedAdaError>;

impl From<anyhow::Error> for StakedAdaError {
    fn from(err: anyhow::Error) -> Self {
        Self::Database { source: err }
    }
}

impl From<serde_json::Error> for StakedAdaError {
    fn from(source: serde_json::Error) -> Self {
        Self::Serialization { source }
    }
}
