//! RBAC Registration Indexing Module

wit_bindgen::generate!({
    world: "hermes:app/hermes",
    path: "../../../wasi/wit",
    inline: "
        package hermes:app;

        world hermes {
            import hermes:cardano/api;
            import hermes:logging/api;
            import hermes:init/api;
            import hermes:sqlite/api;
            
            export hermes:init/event;
            export hermes:cardano/event-on-block;
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
        create::create_rbac_tables,
        data::{rbac_db::RbacDbData, rbac_stake_db::RbacStakeDbData},
        insert::{
            insert_rbac_registration, insert_rbac_stake_address, prepare_insert_rbac_registration,
            prepare_insert_rbac_stake_address,
        },
        open_db_connection,
    },
    utils::log::log_info,
};

use hermes::cardano;

struct RbacRegistrationComponent;

impl exports::hermes::cardano::event_on_block::Guest for RbacRegistrationComponent {
    fn on_cardano_block(
        subscription_id: &exports::hermes::cardano::event_on_block::SubscriptionId,
        block: &exports::hermes::cardano::event_on_block::Block,
    ) {
        const FUNCTION_NAME: &str = "on_cardano_block";
        let registrations = get_rbac_registration(subscription_id.get_network(), block);

        // Early exit if no registration to be added into database
        if registrations.is_empty() {
            return;
        }

        let Ok(sqlite) = open_db_connection() else {
            return;
        };
        let Ok(rbac_stmt) = prepare_insert_rbac_registration(&sqlite) else {
            close_db_connection(sqlite);
            return;
        };
        let Ok(rbac_stake_stmt) = prepare_insert_rbac_stake_address(&sqlite) else {
            close_db_connection(sqlite);
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
                insert_rbac_stake_address(&rbac_stake_stmt, data);
            }
            insert_rbac_registration(&rbac_stmt, rbac_data);
        }
        let _ = rbac_stmt.finalize();
        let _ = rbac_stake_stmt.finalize();

        close_db_connection(sqlite);
    }
}

impl exports::hermes::init::event::Guest for RbacRegistrationComponent {
    fn init() -> bool {
        const FUNCTION_NAME: &str = "init";
        let Ok(sqlite) = open_db_connection() else {
            return false;
        };
        create_rbac_tables(&sqlite);
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

/// Get the RBAC registration from a block.
fn get_rbac_registration(
    network: cardano::api::CardanoNetwork,
    block_resource: &hermes::cardano::api::Block,
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
