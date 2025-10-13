//! Defines API schemas of Cardano network types.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Cardano network type.
// #[derive(Clone, Enum, Debug)]
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub(crate) enum Network {
    /// Cardano mainnet.
    Mainnet,
    /// Cardano preprod.
    Preprod,
    /// Cardano preview.
    Preview,
}

impl From<Network> for cardano_blockchain_types::Network {
    fn from(value: Network) -> Self {
        match value {
            Network::Mainnet => Self::Mainnet,
            Network::Preprod => Self::Preprod,
            Network::Preview => Self::Preview,
        }
    }
}
