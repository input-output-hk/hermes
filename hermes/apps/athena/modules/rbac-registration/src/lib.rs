//! RBAC Registration Module

wit_bindgen::generate!({
    world: "hermes:app/hermes",
    // path: "/home/bkioshn/Work/Catalyst/hermes/wasm/wasi/wit",
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
        }
    ",
    generate_all,
});

use crate::{
    database::{close_db_connection, open_db_connection},
    hermes::{cardano, sqlite::api::Sqlite},
    rbac::get_rbac::{
        get_active_inactive_stake_address, get_rbac_chain_from_cat_id,
        get_rbac_chain_from_stake_address,
    },
    utils::log::{log_error, log_info},
};
use cardano_blockchain_types::{pallas_primitives::Hash, Network, StakeAddress};
use serde_json::json;

export!(RbacRegistrationComponent);

mod database;
mod rbac;
mod utils;

struct RbacRegistrationComponent;

impl exports::hermes::init::event::Guest for RbacRegistrationComponent {
    fn init() -> bool {
        const FUNCTION_NAME: &str = "init";

        let Ok(persistent) = open_db_connection(false) else {
            return false;
        };
        // For volatile table
        // TODO - Change this to in-memory once it is supported
        // <https://github.com/input-output-hk/hermes/issues/553>
        let Ok(volatile) = open_db_connection(false) else {
            return false;
        };

        // Create a network instance
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

        // ----- Get registration chain -----
        // Once the data is indexed, we can get the registration chain from catalyst ID or stake address.
        get_rbac_data(&persistent, &volatile, network, &network_resource);
        close_db_connection(persistent);
        close_db_connection(volatile);
        true
    }
}

fn get_rbac_data(
    persistent: &Sqlite,
    volatile: &Sqlite,
    network: cardano::api::CardanoNetwork,
    network_resource: &cardano::api::Network,
) {
    const FUNCTION_NAME: &str = "get_rbac_data";
    // Testing get rbac data from catalyst id
    // This cat id contain no child registration.
    /* cspell:disable */
    // its stake address `stake_test1urgduxg0zec4zw4k3v33ftsc79ffdwzzj6ka2d3w86dyudqmmj5tv` is inactive
    /* cspell:enable */
    // because other valid registration take over it.
    let cat_id_1 = "preprod.cardano/5HHBcNOAs8uMfQ-II5M3DBXtR0Tp3j3x1GCS6ZxsWzU";
    let rbac_1 =
        get_rbac_chain_from_cat_id(persistent, volatile, cat_id_1, network, network_resource)
            .unwrap()
            .unwrap();
    // No active, 1 inactive
    let (active_1, inactive_1) = get_active_inactive_stake_address(
        rbac_1.stake_addresses(),
        rbac_1.catalyst_id(),
        persistent,
        volatile,
        network,
        network_resource,
    )
    .unwrap();
    log_info(
        file!(),
        FUNCTION_NAME,
        "",
        &format!(
            "📕 From catalyst id {cat_id_1}: Cat ID {}, All stake addresses: {:?}, Active stake address: {active_1:?}, Inactive stake address: {inactive_1:?})",
            rbac_1.catalyst_id(),
            rbac_1.stake_addresses()
        ),
        None,
    );

    /* cspell:disable */
    // Testing get rbac data from stake address
    // `stake_test1urgduxg0zec4zw4k3v33ftsc79ffdwzzj6ka2d3w86dyudqmmj5tv`
    // `e0d0de190f1671513ab68b2314ae18f15296b84296add5362e3e9a4e34`
    // This stake address is taken by `preprod.cardano/ZtnkJZNZHskfS6mhChVstXRrhDPUdzTGwFidSg_YjsA`
    /* cspell:enable */
    let hash: Hash<28> = "d0de190f1671513ab68b2314ae18f15296b84296add5362e3e9a4e34"
        .parse()
        .unwrap();
    let stake_address = StakeAddress::new(Network::Preprod, false, hash.into());

    let rbac_2 = get_rbac_chain_from_stake_address(
        persistent,
        volatile,
        stake_address.clone(),
        network,
        network_resource,
    )
    .unwrap()
    .unwrap();
    // Active 1, No inactive
    let (active_2, inactive_2) = get_active_inactive_stake_address(
        rbac_2.stake_addresses(),
        rbac_2.catalyst_id(),
        persistent,
        volatile,
        network,
        network_resource,
    )
    .unwrap();
    log_info(
        file!(),
        FUNCTION_NAME,
        "",
        &format!(
            "📕 From stake address {stake_address}: Cat ID {}, All stake addresses: {:?}, Active stake address: {active_2:?}, Inactive stake address: {inactive_2:?})",
            rbac_2.catalyst_id(),
            rbac_2.stake_addresses()
        ),
        None,
    );
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
