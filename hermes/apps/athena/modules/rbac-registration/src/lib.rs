//! RBAC Registration Indexing Module
wit_bindgen::generate!({
    world: "hermes:app/hermes",
    path: "../../../wasi/wit",
    inline: "
        package hermes:app;

        world hermes {
            include wasi:cli/imports@0.2.6;
            import hermes:cardano/api;
            import hermes:logging/api;
            import hermes:init/api;
            import hermes:sqlite/api;

            export hermes:init/event;
            export hermes:cardano/event-on-block;
            export hermes:cardano/event-on-immutable-roll-forward;
        }
    ",
    generate_all,
});

export!(RbacRegistrationComponent);

mod database;
mod utils;
use rbac_registration::{
    self,
    cardano::cip509::{Cip0134UriSet, Cip509},
};
use serde_json::json;
use utils::{cardano::block::build_block, log::log_error};

use crate::{
    database::{
        close_db_connection,
        create::{create_rbac_persistent_tables, create_rbac_volatile_tables},
        data::{rbac_db::RbacDbData, rbac_stake_db::RbacStakeDbData},
        delete::{
            roll_back::{prepare_roll_back_delete_from_volatile, roll_back_delete_from_volatile},
            roll_forward::{
                prepare_roll_forward_delete_from_volatile, roll_forward_delete_from_volatile,
            },
        },
        insert::{
            rbac_table::{insert_rbac_registration, prepare_insert_rbac_registration},
            stake_addr_table::{insert_rbac_stake_address, prepare_insert_rbac_stake_address},
        },
        open_db_connection,
        statement::DatabaseStatement,
        RBAC_REGISTRATION_PERSISTENT_TABLE_NAME, RBAC_REGISTRATION_VOLATILE_TABLE_NAME,
        RBAC_STAKE_ADDRESS_PERSISTENT_TABLE_NAME, RBAC_STAKE_ADDRESS_VOLATILE_TABLE_NAME,
    },
    hermes::sqlite::api::Sqlite,
    utils::log::log_info,
};

use hermes::cardano;

struct RbacRegistrationComponent;

impl exports::hermes::cardano::event_on_block::Guest for RbacRegistrationComponent {
    fn on_cardano_block(
        subscription_id: &exports::hermes::cardano::event_on_block::SubscriptionId,
        block: &exports::hermes::cardano::event_on_block::Block,
    ) {
        const FUNC_NAME: &str = "on_cardano_block";

        let registrations = get_rbac_registration(subscription_id.get_network(), block);

        // Early exit if no registration to be added into database
        if registrations.is_empty() {
            return;
        }

        // ------- Open DB Connection -------
        let Ok(sqlite) = open_db_connection(false) else {
            return;
        };
        // Volatile table will be stored in memory
        let Ok(sqlite_in_mem) = open_db_connection(true) else {
            return;
        };

        // ------- Handle Rollback -------
        let Ok(rollback) = block.is_rollback() else {
            return;
        };

        // Rollback occurs
        if rollback {
            handle_rollback(&sqlite_in_mem, block);
        }
        // ------- Prepare persistent Insert into DB -------
        let Ok(rbac_persistent_stmt) =
            prepare_insert_rbac_registration(&sqlite, RBAC_REGISTRATION_PERSISTENT_TABLE_NAME)
        else {
            close_db_connection(sqlite);
            return;
        };
        let Ok(rbac_stake_persistent_stmt) =
            prepare_insert_rbac_stake_address(&sqlite, RBAC_STAKE_ADDRESS_PERSISTENT_TABLE_NAME)
        else {
            close_db_connection(sqlite);
            return;
        };

        // ------- Prepare volatile Insert into DB -------
        let Ok(rbac_volatile_stmt) =
            prepare_insert_rbac_registration(&sqlite_in_mem, RBAC_REGISTRATION_VOLATILE_TABLE_NAME)
        else {
            close_db_connection(sqlite_in_mem);
            return;
        };
        let Ok(rbac_stake_volatile_stmt) = prepare_insert_rbac_stake_address(
            &sqlite_in_mem,
            RBAC_STAKE_ADDRESS_VOLATILE_TABLE_NAME,
        ) else {
            close_db_connection(sqlite_in_mem);
            return;
        };

        // ------- Extract and insert RBAC registrations into DB -------
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
            let stake_addresses = reg
                .certificate_uris()
                .map(Cip0134UriSet::stake_addresses)
                .unwrap_or_default();

            let rbac_data = RbacDbData {
                txn_id: txn_id.clone(),
                catalyst_id: catalyst_id.clone(),
                slot,
                txn_idx,
                prv_txn_id,
                purpose,
                problem_report,
            };

            for stake_address in stake_addresses {
                let data = RbacStakeDbData {
                    stake_address: stake_address.into(),
                    slot,
                    txn_idx,
                    catalyst_id: catalyst_id.clone(),
                    txn_id: txn_id.clone(),
                };
                if block.is_immutable() {
                    insert_rbac_stake_address(&rbac_stake_persistent_stmt, data);
                } else {
                    insert_rbac_stake_address(&rbac_stake_volatile_stmt, data);
                }
            }
            if block.is_immutable() {
                insert_rbac_registration(&rbac_persistent_stmt, rbac_data);
            } else {
                insert_rbac_registration(&rbac_volatile_stmt, rbac_data);
            }
        }

        // ------- Finalize and close DB Connection -------
        DatabaseStatement::finalize_statement(rbac_persistent_stmt, FUNC_NAME);
        DatabaseStatement::finalize_statement(rbac_stake_persistent_stmt, FUNC_NAME);
        DatabaseStatement::finalize_statement(rbac_volatile_stmt, FUNC_NAME);
        DatabaseStatement::finalize_statement(rbac_stake_volatile_stmt, FUNC_NAME);
        close_db_connection(sqlite);
        close_db_connection(sqlite_in_mem);
    }
}

impl exports::hermes::cardano::event_on_immutable_roll_forward::Guest
    for RbacRegistrationComponent
{
    // Immutable roll forward = volatile data become persistent
    fn on_cardano_immutable_roll_forward(
        subscription_id: &exports::hermes::cardano::event_on_block::SubscriptionId,
        block: &exports::hermes::cardano::event_on_block::Block,
    ) {
        const FUNCTION_NAME: &str = "on_cardano_immutable_roll_forward";

        let network_resource = match cardano::api::Network::new(subscription_id.get_network()) {
            Ok(nr) => nr,
            Err(e) => {
                log_error(
                    file!(),
                    FUNCTION_NAME,
                    "cardano::api::Network::new",
                    &format!(
                        "Failed to create network resource {:?}: {e}",
                        subscription_id.get_network()
                    ),
                    None,
                );
                return;
            },
        };
        let (immutable, mutable) = match network_resource.get_tips() {
            Some(tip) => tip,
            None => {
                log_error(
                    file!(),
                    FUNCTION_NAME,
                    "network_resource.get_tips",
                    &format!("Failed to get tips of {:?}", subscription_id.get_network()),
                    None,
                );
                return;
            },
        };

        // Only process immutable roll forward if when it reach tip.
        // Current block is not at the tip, do nothing.
        if mutable != block.get_slot() {
            return;
        }

        let Ok(sqlite) = open_db_connection(false) else {
            return;
        };
        let Ok(sqlite_in_mem) = open_db_connection(true) else {
            return;
        };

        // Given immutable roll forward at 'slot'
        // 1. Indexing the persistent data from the latest slot.
        // 2. Delete all data in volatile table up to `slot`
        let subscribe_from = cardano::api::SyncSlot::Specific(immutable);
        let subscription_id_resource = match network_resource.subscribe_block(subscribe_from) {
            Ok(id) => {
                // Destroy the current subscription
                subscription_id.unsubscribe();
                id
            },
            Err(e) => {
                log_error(
                    file!(),
                    FUNCTION_NAME,
                    "network_resource.subscribe_block",
                    &format!("Failed to subscribe block from {subscribe_from:?}: {e}"),
                    None,
                );
                return;
            },
        };

        // Prepare delete from volatile
        let Ok(rbac_delete_stmt) = prepare_roll_forward_delete_from_volatile(
            &sqlite_in_mem,
            RBAC_REGISTRATION_VOLATILE_TABLE_NAME,
        ) else {
            return;
        };
        let Ok(stake_addr_delete_stmt) = prepare_roll_forward_delete_from_volatile(
            &sqlite_in_mem,
            RBAC_STAKE_ADDRESS_VOLATILE_TABLE_NAME,
        ) else {
            return;
        };
        roll_forward_delete_from_volatile(&rbac_delete_stmt, block.get_slot());
        roll_forward_delete_from_volatile(&stake_addr_delete_stmt, block.get_slot());

        // Finalize and close DB Connection
        DatabaseStatement::finalize_statement(rbac_delete_stmt, FUNCTION_NAME);
        DatabaseStatement::finalize_statement(stake_addr_delete_stmt, FUNCTION_NAME);
        close_db_connection(sqlite);
        close_db_connection(sqlite_in_mem);
    }
}

impl exports::hermes::init::event::Guest for RbacRegistrationComponent {
    fn init() -> bool {
        const FUNCTION_NAME: &str = "init";

        let Ok(sqlite) = open_db_connection(false) else {
            return false;
        };
        // Volatile table will be stored in memory
        let Ok(sqlite_in_mem) = open_db_connection(false) else {
            return false;
        };
        create_rbac_persistent_tables(&sqlite);
        create_rbac_volatile_tables(&sqlite_in_mem);
        close_db_connection(sqlite);

        // Instead of starting from genesis, start from a specific slot just before RBAC data exist.
        let slot = 80374283;
        let subscribe_from = cardano::api::SyncSlot::Specific(slot);
        let network = cardano::api::CardanoNetwork::Preprod;

        let network_resource = match cardano::api::Network::new(network) {
            Ok(nr) => nr,
            Err(e) => {
                log_error(
                    file!(),
                    FUNCTION_NAME,
                    "cardano::api::Network::new",
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
                    file!(),
                    FUNCTION_NAME,
                    "network_resource.subscribe_block",
                    &format!("Failed to subscribe block from {subscribe_from:?}: {e}"),
                    None,
                );
                return false;
            },
        };

        log_info(
            file!(),
            FUNCTION_NAME,
            &format!("💫 Network {network:?}, with subscription id: {subscription_id_resource:?}"),
            "",
            None,
        );

        true
    }
}

/// Handle rollback, rollback just purge data.
fn handle_rollback(
    sqlite: &Sqlite,
    block: &cardano::api::Block,
) {
    let Ok(rollback_rbac_del_stmt) =
        prepare_roll_back_delete_from_volatile(&sqlite, RBAC_REGISTRATION_VOLATILE_TABLE_NAME)
    else {
        return;
    };
    let Ok(rollback_rbac_stake_addr_del_stmt) =
        prepare_roll_back_delete_from_volatile(&sqlite, RBAC_STAKE_ADDRESS_VOLATILE_TABLE_NAME)
    else {
        return;
    };
    roll_back_delete_from_volatile(&rollback_rbac_del_stmt, block.get_slot());
    roll_back_delete_from_volatile(&rollback_rbac_stake_addr_del_stmt, block.get_slot());
}

/// Get the RBAC registration from a block.
fn get_rbac_registration(
    network: cardano::api::CardanoNetwork,
    block_resource: &cardano::api::Block,
) -> Vec<Cip509> {
    const FUNCTION_NAME: &str = "get_rbac_registration";

    let block = match build_block(file!(), FUNCTION_NAME, network, block_resource) {
        Some(b) => b,
        None => return vec![],
    };
    Cip509::from_block(&block, &[])
}

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
