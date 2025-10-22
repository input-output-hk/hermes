//! Command line and environment variable settings for the service

use cardano_blockchain_types::Network;

use super::str_env_var::StringEnvVar;

/// Default chain to follow.
const DEFAULT_NETWORK: NetworkFromStr = NetworkFromStr::Mainnet;

/// Configuration for the chain follower.
#[derive(Clone)]
pub(crate) struct EnvVars {
    /// The Blockchain we sync from.
    pub(crate) chain: Network,
}

#[derive(strum::EnumString, strum::VariantNames, strum::Display)]
#[strum(ascii_case_insensitive)]
enum NetworkFromStr {
    /// Mainnet
    Mainnet,
    /// Preprod
    Preprod,
    /// Preview
    Preview,
    /// Devnet
    Devnet,
}

impl From<NetworkFromStr> for Network {
    fn from(value: NetworkFromStr) -> Self {
        match value {
            NetworkFromStr::Mainnet => Self::Mainnet,
            NetworkFromStr::Preprod => Self::Preprod,
            NetworkFromStr::Preview => Self::Preview,
            NetworkFromStr::Devnet => Self::Devnet {
                genesis_key: "5b33322c3235332c3138362c3230312c3137372c31312c3131372c3133352c3138372c3136372c3138312c3138382c32322c35392c3230362c3130352c3233312c3135302c3231352c33302c37382c3231322c37362c31362c3235322c3138302c37322c3133342c3133372c3234372c3136312c36385d",
                magic: 42,
                network_id: 0,
                byron_epoch_length: 100_000,
                byron_slot_length: 1000,
                byron_known_slot: 0,
                byron_known_time: 1_564_010_416,
                byron_known_hash: "8f8602837f7c6f8b8867dd1cbc1842cf51a27eaed2c70ef48325d00f8efb320f",
                shelley_epoch_length: 100,
                shelley_slot_length: 1,
                shelley_known_slot: 1_598_400,
                shelley_known_hash: "02b1c561715da9e540411123a6135ee319b02f60b9a11a603d3305556c04329f",
                shelley_known_time: 1_595_967_616,
            },
        }
    }
}

impl EnvVars {
    /// Create a config for a cassandra cluster, identified by a default namespace.
    pub(super) fn new() -> Self {
        let chain = StringEnvVar::new_as_enum("CHAIN_NETWORK", DEFAULT_NETWORK, false).into();

        Self { chain }
    }
}
