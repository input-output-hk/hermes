//! Cardano utility module

pub mod block;

use serde_json::json;

use crate::{bindings::hermes::cardano, utils::log::log_error};

impl From<cardano::api::CardanoNetwork> for cardano_blockchain_types::Network {
    fn from(network: cardano::api::CardanoNetwork) -> cardano_blockchain_types::Network {
        match network {
            cardano::api::CardanoNetwork::Mainnet => cardano_blockchain_types::Network::Mainnet,
            cardano::api::CardanoNetwork::Preprod => cardano_blockchain_types::Network::Preprod,
            cardano::api::CardanoNetwork::Preview => cardano_blockchain_types::Network::Preview,
            cardano::api::CardanoNetwork::TestnetMagic(n) => {
                // TODO(bkioshn) - This should be mapped to
                // cardano_blockchain_types::Network::Devnet
                log_error(
                    file!(),
                    "From<cardano::api::CardanoNetwork> for cardano_blockchain_types::Network",
                    "cardano::api::CardanoNetwork::TestnetMagic",
                    "Unsupported network",
                    Some(&json!({ "network": format!("TestnetMagic {n}") }).to_string()),
                );
                panic!("Unsupported network");
            },
        }
    }
}
