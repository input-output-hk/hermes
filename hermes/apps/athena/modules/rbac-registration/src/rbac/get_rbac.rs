//! Get the RBAC chain by Catalyst ID or stake address.

use std::collections::HashSet;

use cardano_blockchain_types::StakeAddress;
use catalyst_types::catalyst_id::CatalystId;
use rbac_registration::registration::cardano::RegistrationChain;

use crate::{
    database::select::{
        cat_id::select_rbac_registration_chain_from_cat_id,
        stake_addr::select_rbac_registration_chain_from_stake_addr,
    },
    hermes::{
        cardano::api::{CardanoNetwork, Network},
        sqlite::api::Sqlite,
    },
    rbac::build_rbac_chain::build_registration_chain,
    utils::log::log_error,
};

/// Get the RBAC chain by Catalyst ID.
pub(crate) fn get_rbac_chain_from_cat_id(
    sqlite: &Sqlite,
    sqlite_in_mem: &Sqlite,
    cat_id: &str,
    network: CardanoNetwork,
    network_resource: &Network,
) -> anyhow::Result<Option<RegistrationChain>> {
    let chain_info = select_rbac_registration_chain_from_cat_id(sqlite, sqlite_in_mem, cat_id)?;

    let reg_chain = build_registration_chain(network, network_resource, chain_info)?;
    Ok(reg_chain)
}

/// Get the RBAC chain by Stake address.
pub(crate) fn get_rbac_chain_from_stake_address(
    sqlite: &Sqlite,
    sqlite_in_mem: &Sqlite,
    stake_address: StakeAddress,
    network: CardanoNetwork,
    network_resource: &Network,
) -> anyhow::Result<Option<RegistrationChain>> {
    let chain_info =
        select_rbac_registration_chain_from_stake_addr(sqlite, sqlite_in_mem, stake_address)?;
    let reg_chain = build_registration_chain(network, network_resource, chain_info)?;
    Ok(reg_chain)
}

/// Active stake addresses type.
type Active = Vec<StakeAddress>;

/// Inactive stake addresses type.
type Inactive = Vec<StakeAddress>;

/// Get the active and inactive stake addresses given Catalyst ID.
pub(crate) fn get_active_inactive_stake_address(
    stake_addresses: HashSet<StakeAddress>,
    cat_id: &CatalystId,
    sqlite: &Sqlite,
    sqlite_in_mem: &Sqlite,
    network: CardanoNetwork,
    network_resource: &Network,
) -> anyhow::Result<(Active, Inactive)> {
    const FUNCTION_NAME: &str = "get_active_inactive_stake_address";
    let mut active_stake_addresses: Vec<StakeAddress> = Vec::new();
    let mut inactive_stake_addresses: Vec<StakeAddress> = Vec::new();
    for s in stake_addresses {
        let chain_info =
            select_rbac_registration_chain_from_stake_addr(sqlite, sqlite_in_mem, s.clone())?;
        let reg_chain = build_registration_chain(network, network_resource, chain_info)?;
        // There should be a chain associated with the stake address, since the stake address
        // is extracted from the valid registration chain.
        if let Some(r) = reg_chain {
            if r.catalyst_id() == cat_id {
                active_stake_addresses.push(s);
            } else {
                inactive_stake_addresses.push(s);
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
