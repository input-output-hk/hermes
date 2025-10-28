//! Cardano utility module

pub mod block;

use crate::bindings::hermes::cardano;

impl From<cardano::api::CardanoNetwork> for cardano_blockchain_types::Network {
    fn from(network: cardano::api::CardanoNetwork) -> cardano_blockchain_types::Network {
        match network {
            cardano::api::CardanoNetwork::Mainnet => cardano_blockchain_types::Network::Mainnet,
            cardano::api::CardanoNetwork::Preprod => cardano_blockchain_types::Network::Preprod,
            cardano::api::CardanoNetwork::Preview => cardano_blockchain_types::Network::Preview,
            cardano::api::CardanoNetwork::TestnetMagic(n) => {
                // TODO(bkioshn) - This should be mapped to
                // cardano_blockchain_types::Network::Devnet
                let err = format!("Unsupported network TestnetMagic {n}");
                panic!("{err}");
            },
        }
    }
}

impl From<cardano_blockchain_types::Network> for cardano::api::CardanoNetwork {
    fn from(network: cardano_blockchain_types::Network) -> cardano::api::CardanoNetwork {
        match network {
            cardano_blockchain_types::Network::Mainnet => cardano::api::CardanoNetwork::Mainnet,
            cardano_blockchain_types::Network::Preprod => cardano::api::CardanoNetwork::Preprod,
            cardano_blockchain_types::Network::Preview => cardano::api::CardanoNetwork::Preview,
            cardano_blockchain_types::Network::Devnet { magic, .. } => {
                cardano::api::CardanoNetwork::TestnetMagic(magic)
            },
            _ => {
                // Handle any future variants added due to #[non_exhaustive]
                let err = format!("Unknown network variant {network:?}");
                panic!("{err}");
            },
        }
    }
}
