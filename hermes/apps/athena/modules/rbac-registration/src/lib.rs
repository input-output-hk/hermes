// Allow everything since this is generated code.
#![allow(clippy::all, unused)]

mod hermes;
mod stub;

use cardano_blockchain_types;
use catalyst_types::catalyst_id::CatalystId;
use rbac_registration;
use rbac_registration::cardano::cip509::Cip509;
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
        if !registrations.is_empty() {
            log(
                hermes::hermes::logging::api::Level::Trace,
                FILE_NAME,
                "",
                &format!("🦄 RBAC registration: {registrations:?}").as_str(),
                "",
                None,
            );
        }
    }
}

impl hermes::exports::hermes::init::event::Guest for RbacRegistrationComponent {
    fn init() -> bool {
        let subscribe_from = hermes::hermes::cardano::api::SyncSlot::Specific(87374283);
        let network = hermes::hermes::cardano::api::CardanoNetwork::Preprod;

        let network_resource = hermes::hermes::cardano::api::Network::new(network).unwrap();
        let subscription_id_resource = network_resource.subscribe_block(subscribe_from).unwrap();
        log(
            hermes::hermes::logging::api::Level::Trace,
            FILE_NAME,
            "",
            &format!("🎧 Network {network:?}, with subscription id: {subscription_id_resource:?}")
                .as_str(),
            "",
            None,
        );
        true
    }
}

enum RbacSelection {
    All,
    CatId(String),
    StakeAddress(String),
}

/// Filter the RBAC registrations based on the selection.
fn filter_rbac_registration(
    selection: RbacSelection,
    rbac_registrations: Vec<Cip509>,
) -> Vec<Cip509> {
    const FUNCTION_NAME: &str = "filter_rbac_registration";
    match selection {
        RbacSelection::All => rbac_registrations,
        RbacSelection::CatId(cat_id) => match cat_id.parse::<CatalystId>() {
            Ok(parsed_id) => rbac_registrations
                .into_iter()
                .filter(|rbac_reg| rbac_reg.catalyst_id() == Some(&parsed_id))
                .collect(),
            Err(_) => {
                log(
                    hermes::hermes::logging::api::Level::Error,
                    FILE_NAME,
                    FUNCTION_NAME,
                    "cat_id.parse::<CatalystId>()",
                    "Failed to parse CatalystId from string",
                    Some(json!({ "cat_id": cat_id  }).to_string()),
                );
                vec![]
            },
        },
        StakeAddress(stake_address) => rbac_registrations.into_iter()
            .into_iter()
            .filter(|rbac_reg| rbac_reg.stake_address == stake_address)
            .collect(),
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
                Some(json!({ "slot": pallas_block.slot() }).to_string()),
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

fn log(
    level: hermes::hermes::logging::api::Level,
    file: &str,
    function: &str,
    context: &str,
    msg: &str,
    data: Option<hermes::hermes::json::api::Json>,
) {
    hermes::hermes::logging::api::log(
        level,
        Some(file),
        Some(function),
        None,
        None,
        Some(context),
        msg,
        None,
    );
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
            hermes::hermes::cardano::api::CardanoNetwork::TestnetMagic(_) => todo!(),
        }
    }
}

hermes::export!(RbacRegistrationComponent with_types_in hermes);
