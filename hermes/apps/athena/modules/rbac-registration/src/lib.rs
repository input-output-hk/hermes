// Allow everything since this is generated code.
#![allow(clippy::all, unused)]
pub(crate) mod database;
mod hermes;
mod stub;
mod utils;

use rbac_registration::{self, cardano::cip509::Cip509};
use serde_json::json;
use utils::log::log_error;

use crate::{
    database::{
        create::create_rbac_tables,
        data::RbacDbData,
        insert::{insert_rbac_registration, prepare_insert_rbac_registration},
    },
    utils::log::log_info,
};

const FILE_NAME: &str = "rbac-registration/src/lib.rs";

struct RbacRegistrationComponent;

impl hermes::exports::hermes::cardano::event_on_block::Guest for RbacRegistrationComponent {
    fn on_cardano_block(
        subscription_id: &hermes::exports::hermes::cardano::event_on_block::SubscriptionId,
        block: &hermes::exports::hermes::cardano::event_on_block::Block,
    ) {
        let stmt = prepare_insert_rbac_registration().unwrap();

        let registrations =
            get_rbac_registration(block.raw(), subscription_id.get_network(), block.get_fork());

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

            let data = RbacDbData {
                txn_id: txn_id.into(),
                catalyst_id: cat_id.map(|id| id.as_short_id().to_string()),
                slot: slot.into(),
                txn_idx: txn_idx.into(),
                prv_txn_id: prv_txn_id.map(|id| id.into()),
                purpose: purpose.map(|p| p.to_string()),
                problem_report: problem_report
                    .is_problematic()
                    .then(|| serde_json::to_string(&problem_report).ok())
                    .flatten(),
            };
            log_info(FILE_NAME, "", &format!("💫 {data:?}"), "", None);
            insert_rbac_registration(&stmt, data);
        }

        stmt.finalize().unwrap();
    }
}

impl hermes::exports::hermes::init::event::Guest for RbacRegistrationComponent {
    fn init() -> bool {
        const FUNCTION_NAME: &str = "init";

        create_rbac_tables();

        let slot = 87374283;
        let subscribe_from = hermes::hermes::cardano::api::SyncSlot::Specific(slot);
        let network = hermes::hermes::cardano::api::CardanoNetwork::Preprod;

        let network_resource = match hermes::hermes::cardano::api::Network::new(network) {
            Ok(nr) => nr,
            Err(e) => {
                log_error(
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
                log_error(
                    FILE_NAME,
                    FUNCTION_NAME,
                    "network_resource.subscribe_block",
                    &format!("Failed to subscribe block from {subscribe_from:?}: {e}"),
                    None,
                );
                return false;
            },
        };

        log_info(
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
                log_error(
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
            log_error(
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
            log_error(
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
            log_error(
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
                log_error(
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

hermes::export!(RbacRegistrationComponent with_types_in hermes);
