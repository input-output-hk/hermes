use std::collections::HashMap;

use cardano_blockchain_types::Network;
use catalyst_types::catalyst_id::role_index::RoleId;

use crate::{
    database::open_db_connection,
    hermes::cardano::{self, api::CardanoNetwork},
    rbac::get_rbac::{get_rbac_chain_from_cat_id, get_rbac_chain_from_stake_address},
    service::api::registration_get::v1::{
        cat_id_or_stake::CatIdOrStake, chain_info::ChainInfo,
        registration_chain::RbacRegistrationChain, response::ResponsesV1, role_data::RbacRoleData,
    },
};

/// Get RBAC registration V1 endpoint.
pub fn endpoint_v1(
    lookup: Option<CatIdOrStake>,
    network: CardanoNetwork,
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

    match lookup {
        Some(CatIdOrStake::CatId(cat_id)) => {
            let (reg_chain, metadata) = match get_rbac_chain_from_cat_id(
                &persistent,
                &volatile,
                cat_id,
                network,
                &network_resource,
            ) {
                Ok(Some((chain, meta))) => (chain, meta),
                Ok(None) => {
                    return ResponsesV1::NotFound;
                },
                Err(e) => {
                    return ResponsesV1::UnprocessableContent(e.to_string());
                },
            };

            let chain_info = ChainInfo {
                chain: reg_chain,
                last_persistent_txn: metadata.last_persistent_txn.map(Into::into),
                last_volatile_txn: metadata.last_volatile_txn.map(Into::into),
                last_persistent_slot: metadata.last_persistent_slot,
                network: network.into(),
            };

            match RbacRegistrationChain::new(&chain_info) {
                Ok(rbac_registration_chain) => ResponsesV1::Ok(rbac_registration_chain),
                Err(e) => ResponsesV1::InternalServerError(e.to_string()),
            }
        },
        Some(CatIdOrStake::Address(stake_address)) => {
            let (reg_chain, metadata) = match get_rbac_chain_from_stake_address(
                &persistent,
                &volatile,
                &stake_address,
                network,
                &network_resource,
            ) {
                Ok(Some((chain, meta))) => (chain, meta),
                Ok(None) => {
                    return ResponsesV1::NotFound;
                },
                Err(e) => {
                    return ResponsesV1::UnprocessableContent(e.to_string());
                },
            };

            let chain_info = ChainInfo {
                chain: reg_chain,
                last_persistent_txn: metadata.last_persistent_txn.map(Into::into),
                last_volatile_txn: metadata.last_volatile_txn.map(Into::into),
                last_persistent_slot: metadata.last_persistent_slot,
                network: network.into(),
            };

            match RbacRegistrationChain::new(&chain_info) {
                Ok(rbac_registration_chain) => ResponsesV1::Ok(rbac_registration_chain),
                Err(e) => ResponsesV1::InternalServerError(e.to_string()),
            }
        },
        // TODO - Need to be handle with auth token
        None => ResponsesV1::UnprocessableContent("Lookup parameter is required".to_string()),
    }
}

/// Gets and converts a role data from the given chain info.
fn role_data(
    info: &ChainInfo,
    network: Network,
) -> anyhow::Result<HashMap<RoleId, RbacRoleData>> {
    info.chain
        .role_data_history()
        .iter()
        .map(|(&role, data)| {
            RbacRoleData::new(data, info.last_persistent_slot, &info.chain, network)
                .map(|rbac| (role, rbac))
        })
        .collect()
}
