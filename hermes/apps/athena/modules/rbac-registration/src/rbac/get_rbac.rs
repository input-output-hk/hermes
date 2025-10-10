//! Get the RBAC chain by Catalyst ID or stake address.

use std::collections::HashSet;

use cardano_blockchain_types::StakeAddress;
use catalyst_types::catalyst_id::CatalystId;
use rbac_registration::registration::cardano::RegistrationChain;
use shared::{
    bindings::hermes::{
        cardano::api::{CardanoNetwork, Network},
        sqlite::api::Sqlite,
    },
    utils::log::log_error,
};

use crate::{
    database::select::{
        cat_id::select_rbac_registration_chain_from_cat_id,
        stake_addr::select_rbac_registration_chain_from_stake_addr,
    },
    rbac::{build_rbac_chain::build_registration_chain, rbac_chain_metadata::RbacChainMetadata},
    service::common::types::cardano::cip19_stake_address::Cip19StakeAddress,
};

/// Get the RBAC chain by Catalyst ID.
pub(crate) fn get_rbac_chain_from_cat_id(
    persistent: &Sqlite,
    volatile: &Sqlite,
    cat_id: &CatalystId,
    network: CardanoNetwork,
    network_resource: &Network,
) -> anyhow::Result<Option<(RegistrationChain, RbacChainMetadata)>> {
    let (reg_locations, metadata) =
        select_rbac_registration_chain_from_cat_id(persistent, volatile, &cat_id.to_string())?;

    let reg_chain = build_registration_chain(network, network_resource, reg_locations)?;
    if reg_chain.is_none() {
        return Ok(None);
    }
    Ok(reg_chain.map(|chain| (chain, metadata)))
}

/// Get the RBAC chain by Stake address.
pub(crate) fn get_rbac_chain_from_stake_address(
    persistent: &Sqlite,
    volatile: &Sqlite,
    stake_address: Cip19StakeAddress,
    network: CardanoNetwork,
    network_resource: &Network,
) -> anyhow::Result<Option<(RegistrationChain, RbacChainMetadata)>> {
    let stake_address: StakeAddress = stake_address.try_into()?;

    let (reg_locations, metadata) =
        select_rbac_registration_chain_from_stake_addr(persistent, volatile, stake_address)?;
    let reg_chain = build_registration_chain(network, network_resource, reg_locations)?;
    if reg_chain.is_none() {
        return Ok(None);
    }
    Ok(reg_chain.map(|chain| (chain, metadata)))
}

// TODO: These is part of the v2 api so dead_code is okay
/// Active stake addresses type.
#[allow(dead_code)]
type Active = HashSet<StakeAddress>;

/// Inactive stake addresses type.
#[allow(dead_code)]
type Inactive = HashSet<StakeAddress>;

/// Get the active and inactive stake addresses given Catalyst ID.
#[allow(dead_code)]
pub(crate) fn get_active_inactive_stake_address(
    stake_addresses: HashSet<StakeAddress>,
    cat_id: &CatalystId,
    persistent: &Sqlite,
    volatile: &Sqlite,
    network: CardanoNetwork,
    network_resource: &Network,
) -> anyhow::Result<(Active, Inactive)> {
    const FUNCTION_NAME: &str = "get_active_inactive_stake_address";
    let mut active_stake_addresses = Active::new();
    let mut inactive_stake_addresses = Inactive::new();
    for s in stake_addresses {
        let (reg_locations, _) =
            select_rbac_registration_chain_from_stake_addr(persistent, volatile, s.clone())?;
        let reg_chain = build_registration_chain(network, network_resource, reg_locations)?;
        // There should be a chain associated with the stake address, since the stake address
        // is extracted from the valid registration chain.
        if let Some(r) = reg_chain {
            if r.catalyst_id() == cat_id {
                active_stake_addresses.insert(s);
            } else {
                inactive_stake_addresses.insert(s);
            }
        } else {
            let error = format!("There should be a chain associated with the stake address {s}");
            log_error(
                file!(),
                FUNCTION_NAME,
                "build_registration_chain",
                &error,
                None,
            );
            anyhow::bail!(error);
        }
    }
    Ok((active_stake_addresses, inactive_stake_addresses))
}
