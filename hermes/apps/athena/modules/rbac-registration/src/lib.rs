// Allow everything since this is generated code.

mod hermes;
mod stub;

use rbac_registration::{self, cardano::cip509::Cip509};
use serde_json::json;

const FILE_NAME: &str = "rbac-registration/lib.rs";

struct RbacRegistrationComponent;

impl hermes::exports::hermes::cardano::event_on_block::Guest for RbacRegistrationComponent {
    fn on_cardano_block(
        subscription_id: &hermes::exports::hermes::cardano::event_on_block::SubscriptionId,
        block: &hermes::exports::hermes::cardano::event_on_block::Block,
    ) {
        let registrations =
            get_rbac_registration(block.raw(), subscription_id.get_network(), block.get_fork());

        #[allow(unused_variables)]
        for reg in registrations {
            // Data needed for db
            let txn_id = reg.txn_hash();
            let cat_id = reg.catalyst_id();
            let slot = reg.origin().point().slot_or_default();
            let txn_idx = reg.origin().txn_index();
            let purpose = reg.purpose();
            let prv_txn_id = reg.previous_transaction();
            let problem_report = reg.report();
            // Can contain multiple stake addresses
            let stake_addresses = reg.role_0_stake_addresses();

            log(
                hermes::hermes::logging::api::Level::Trace,
                FILE_NAME,
                "",
                &format!("🦄 RBAC registration:r {reg:?}"),
                "",
                None,
            );
        }
    }
}

impl hermes::exports::hermes::init::event::Guest for RbacRegistrationComponent {
    fn init() -> bool {
        const FUNCTION_NAME: &str = "init";

        let slot = 87374283;
        let subscribe_from = hermes::hermes::cardano::api::SyncSlot::Specific(slot);
        let network = hermes::hermes::cardano::api::CardanoNetwork::Preprod;

        let network_resource = match hermes::hermes::cardano::api::Network::new(network) {
            Ok(nr) => nr,
            Err(e) => {
                log(
                    hermes::hermes::logging::api::Level::Error,
                    FILE_NAME,
                    FUNCTION_NAME,
                    "hermes::hermes::cardano::api::Network::new",
                    &format!("Failed to create network resource {network:?}: {e}"),
                    None,
                );
                return false;
            },
        };

        let subscription_id_resource = match network_resource.subscribe_block(subscribe_from) {
            Ok(id) => id,
            Err(e) => {
                log(
                    hermes::hermes::logging::api::Level::Error,
                    FILE_NAME,
                    FUNCTION_NAME,
                    "network_resource.subscribe_block",
                    &format!("Failed to subscribe block from {subscribe_from:?}: {e}"),
                    None,
                );
                return false;
            },
        };

        log(
            hermes::hermes::logging::api::Level::Trace,
            FILE_NAME,
            FUNCTION_NAME,
            &format!("💫 Network {network:?}, with subscription id: {subscription_id_resource:?}"),
            "",
            None,
        );
        true
    }
}

/// Get the RBAC registration from a block.
fn get_rbac_registration(
    raw_block: Vec<u8>,
    network: hermes::hermes::cardano::api::CardanoNetwork,
    fork_counter: u64,
) -> Vec<Cip509> {
    const FUNCTION_NAME: &str = "get_rbac_reg";
    // Create a pallas block from a raw block data
    let pallas_block =
        match cardano_blockchain_types::pallas_traverse::MultiEraBlock::decode(&raw_block) {
            Ok(block) => block,
            Err(_) => {
                log(
                    hermes::hermes::logging::api::Level::Error,
                    FILE_NAME,
                    FUNCTION_NAME,
                    "pallas_traverse::MultiEraBlock::decode",
                    "Failed to decode pallas block from raw block data",
                    None,
                );
                return vec![];
            },
        };

    let prv_slot = match pallas_block.slot().checked_sub(1) {
        Some(slot) => slot,
        None => {
            log(
                hermes::hermes::logging::api::Level::Error,
                FILE_NAME,
                FUNCTION_NAME,
                "pallas_block.slot().checked_sub()",
                "Slot underflow when computing previous point",
                Some(&json!({ "slot": pallas_block.slot() }).to_string()),
            );
            return vec![];
        },
    };

    let prv_hash = match pallas_block.header().previous_hash() {
        Some(hash) => hash,
        None => {
            log(
                hermes::hermes::logging::api::Level::Error,
                FILE_NAME,
                FUNCTION_NAME,
                "pallas_block.header().previous_hash()",
                "Missing previous hash in block header",
                None,
            );
            return vec![];
        },
    };

    // Need previous point in order to construct our multi-era block
    let prv_point = cardano_blockchain_types::Point::new(prv_slot.into(), prv_hash.into());

    // Construct our version of multi-era block
    let block = match cardano_blockchain_types::MultiEraBlock::new(
        network.into(),
        raw_block,
        &prv_point,
        fork_counter.into(),
    ) {
        Ok(block) => block,
        Err(_) => {
            log(
                hermes::hermes::logging::api::Level::Error,
                FILE_NAME,
                FUNCTION_NAME,
                "cardano_blockchain_types::MultiEraBlock::new",
                "Failed to construct multi-era block",
                None,
            );
            return vec![];
        },
    };

    Cip509::from_block(&block, &[])
}

impl From<hermes::hermes::cardano::api::CardanoNetwork> for cardano_blockchain_types::Network {
    fn from(
        network: hermes::hermes::cardano::api::CardanoNetwork
    ) -> cardano_blockchain_types::Network {
        match network {
            hermes::hermes::cardano::api::CardanoNetwork::Mainnet => {
                cardano_blockchain_types::Network::Mainnet
            },
            hermes::hermes::cardano::api::CardanoNetwork::Preprod => {
                cardano_blockchain_types::Network::Preprod
            },
            hermes::hermes::cardano::api::CardanoNetwork::Preview => {
                cardano_blockchain_types::Network::Preview
            },
            hermes::hermes::cardano::api::CardanoNetwork::TestnetMagic(n) => {
                // TODO(bkioshn) - This should be mapped to
                // cardano_blockchain_types::Network::Devnet
                log(
                    hermes::hermes::logging::api::Level::Error,
                    FILE_NAME,
                    "From<hermes::hermes::cardano::api::CardanoNetwork> for cardano_blockchain_types::Network",
                    "hermes::hermes::cardano::api::CardanoNetwork::TestnetMagic",
                    "Unsupported network",
                    Some(&json!({ "network": format!("TestnetMagic {n}") }).to_string()),
                );
                panic!("Unsupported network");
            },
        }
    }
}

/// Logging helper.
fn log(
    level: hermes::hermes::logging::api::Level,
    file: &str,
    function: &str,
    context: &str,
    msg: &str,
    data: Option<&str>,
) {
    hermes::hermes::logging::api::log(
        level,
        Some(file),
        Some(function),
        None,
        None,
        Some(context),
        msg,
        data,
    );
}

hermes::export!(RbacRegistrationComponent with_types_in hermes);
