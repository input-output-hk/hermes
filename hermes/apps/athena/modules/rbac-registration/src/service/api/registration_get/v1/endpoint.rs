//! Implementation of the GET `/rbac/registrations` V1 endpoint.

use shared::{bindings::hermes::cardano, extract_header, utils::sqlite::open_db_connection};

use crate::{
    hermes::http_gateway::api::Headers,
    rbac::{
        get_rbac::{get_rbac_chain_from_cat_id, get_rbac_chain_from_stake_address},
        token::CatalystRBACTokenV1,
    },
    service::api::registration_get::v1::{
        cat_id_or_stake::CatIdOrStake, chain_info::ChainInfo,
        registration_chain::RbacRegistrationChain, response::ResponsesV1,
    },
};

/// Get RBAC registration V1 endpoint.
pub fn endpoint_v1(
    mut lookup: Option<String>,
    network: cardano::api::CardanoNetwork,
    headers: &Headers,
) -> ResponsesV1 {
    let persistent = match open_db_connection(false) {
        Ok(db) => db,
        Err(e) => return ResponsesV1::ServiceUnavailable(e.to_string()),
    };
    // For volatile table
    // TODO - Change this to in-memory once it is supported
    // <https://github.com/input-output-hk/hermes/issues/553>
    let volatile = match open_db_connection(false) {
        Ok(db) => db,
        Err(e) => return ResponsesV1::ServiceUnavailable(e.to_string()),
    };

    let network_resource = match cardano::api::Network::new(network) {
        Ok(nr) => nr,
        Err(e) => return ResponsesV1::ServiceUnavailable(e.to_string()),
    };
    // No lookup provided, use the Bearer token to search for registration
    if lookup.is_none()
        && let Some(token) = extract_header!(headers, "Authorization", "Bearer")
    {
        // Tokens processed by the auth module should always be valid at this point.
        // If parsing fails, it means the auth module wasn’t involved (validation disabled)
        lookup = match CatalystRBACTokenV1::parse(&token) {
            Ok(t) => Some(t.catalyst_id().as_short_id().to_string()),
            Err(e) => {
                return ResponsesV1::PreconditionFailed(format!("failed to parse token: {e}"));
            },
        };
    }

    let parsed_lookup = match lookup {
        Some(lookup_str) => {
            match CatIdOrStake::try_from(lookup_str.as_str()) {
                Ok(cat_id_or_stake) => cat_id_or_stake,
                Err(e) => {
                    return ResponsesV1::PreconditionFailed(format!(
                        "failed to parse parameter `lookup`: {e}",
                    ));
                },
            }
        },
        None => {
            return ResponsesV1::UnprocessableContent(
                "Either lookup parameter or token must be provided".to_string(),
            );
        },
    };

    match parsed_lookup {
        CatIdOrStake::CatId(cat_id) => {
            let (reg_chain, metadata) = match get_rbac_chain_from_cat_id(
                &persistent,
                &volatile,
                &cat_id,
                network,
                &network_resource,
            ) {
                Ok(Some((chain, meta))) => (chain, meta),
                Ok(None) => {
                    return ResponsesV1::NotFound;
                },
                Err(e) => {
                    return ResponsesV1::InternalServerError(e.to_string());
                },
            };

            let chain_info = ChainInfo {
                chain: reg_chain,
                last_persistent_txn: metadata.last_persistent_txn,
                last_volatile_txn: metadata.last_volatile_txn,
                last_persistent_slot: metadata.last_persistent_slot,
                network: network.into(),
            };

            match RbacRegistrationChain::new(&chain_info) {
                Ok(rbac_registration_chain) => ResponsesV1::Ok(rbac_registration_chain),
                Err(e) => ResponsesV1::InternalServerError(e.to_string()),
            }
        },
        CatIdOrStake::Address(stake_address) => {
            let (reg_chain, metadata) = match get_rbac_chain_from_stake_address(
                &persistent,
                &volatile,
                stake_address,
                network,
                &network_resource,
            ) {
                Ok(Some((chain, meta))) => (chain, meta),
                Ok(None) => {
                    return ResponsesV1::NotFound;
                },
                Err(e) => {
                    return ResponsesV1::InternalServerError(e.to_string());
                },
            };

            let chain_info = ChainInfo {
                chain: reg_chain,
                last_persistent_txn: metadata.last_persistent_txn,
                last_volatile_txn: metadata.last_volatile_txn,
                last_persistent_slot: metadata.last_persistent_slot,
                network: network.into(),
            };

            match RbacRegistrationChain::new(&chain_info) {
                Ok(rbac_registration_chain) => ResponsesV1::Ok(rbac_registration_chain),
                Err(e) => ResponsesV1::InternalServerError(e.to_string()),
            }
        },
    }
}
