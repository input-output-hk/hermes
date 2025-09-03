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
        close_db_connection,
        create::create_rbac_tables,
        data::{rbac_db::RbacDbData, rbac_stake_db::RbacStakeDbData},
        insert::{
            insert_rbac_registration, insert_rbac_stake_address, prepare_insert_rbac_registration,
            prepare_insert_rbac_stake_address, RBAC_INSERT_RBAC_REGISTRATION,
            RBAC_INSERT_STAKE_ADDRESS,
        },
        open_db_connection,
        select::select_rbac_root_registration_from_cat_id,
        SQLITE,
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
        let registrations =
            get_rbac_registration(block.raw(), subscription_id.get_network(), block.get_fork());

        // Early exit if no registration to be added into database
        if registrations.is_empty() {
            return;
        }

        let Ok(sqlite) = open_db_connection() else {
            return;
        };
        let Ok(rbac_stmt) = prepare_insert_rbac_registration(&sqlite) else {
            return;
        };
        let Ok(rbac_stake_stmt) = prepare_insert_rbac_stake_address(&sqlite) else {
            return;
        };

        for reg in registrations.clone() {

            // Data needed for db
            let txn_id: Vec<u8> = reg.txn_hash().into();
            let catalyst_id: Option<String> =
                reg.catalyst_id().map(|id| id.as_short_id().to_string());
            let slot: u64 = reg.origin().point().slot_or_default().into();
            let txn_idx: u16 = reg.origin().txn_index().into();
            let purpose: Option<String> = reg.purpose().map(|p| p.to_string());
            let prv_txn_id: Option<Vec<u8>> = reg.previous_transaction().map(|p| p.into());
            let problem_report: Option<String> = reg
                .report()
                .is_problematic()
                .then(|| serde_json::to_string(&reg.report()).ok())
                .flatten();
            // Can contain multiple stake addresses
            let stake_addresses = reg.role_0_stake_addresses();

            let data = RbacDbData {
                txn_id,
                catalyst_id: catalyst_id.clone(),
                slot,
                txn_idx,
                prv_txn_id,
                purpose,
                problem_report,
            };
            insert_rbac_registration(&rbac_stmt, data);

            for stake_address in stake_addresses {
                let data = RbacStakeDbData {
                    stake_address: stake_address.into(),
                    slot,
                    txn_idx,
                    catalyst_id: catalyst_id.clone(),
                };
                insert_rbac_stake_address(&rbac_stake_stmt, data);
            }
        }
        rbac_stmt.finalize();
        rbac_stake_stmt.finalize();

        close_db_connection(sqlite);
    }
}

impl hermes::exports::hermes::init::event::Guest for RbacRegistrationComponent {
    fn init() -> bool {
        const FUNCTION_NAME: &str = "init";

        let Ok(sqlite) = open_db_connection() else {
            return false;
        };
        create_rbac_tables(&sqlite);
        close_db_connection(sqlite);
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
    const FUNCTION_NAME: &str = "get_rbac_registration";
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
