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
    utils::log::log_info,
};

/// Get the RBAC chain by Catalyst ID.
pub(crate) fn get_rbac_chain_from_cat_id(
    sqlite: &Sqlite,
    cat_id: &str,
    network: CardanoNetwork,
    network_resource: &Network,
) -> anyhow::Result<Option<RegistrationChain>> {
    let chain_info = select_rbac_registration_chain_from_cat_id(sqlite, cat_id)?;

    let reg_chain = build_registration_chain(network, network_resource, chain_info)?;
    Ok(reg_chain)
}

/// Get the RBAC chain by Stake address.
pub(crate) fn get_rbac_chain_from_stake_address(
    sqlite: &Sqlite,
    stake_address: StakeAddress,
    network: CardanoNetwork,
    network_resource: &Network,
) -> anyhow::Result<Option<RegistrationChain>> {
    let chain_info = select_rbac_registration_chain_from_stake_addr(sqlite, stake_address)?;
    let reg_chain = build_registration_chain(network, network_resource, chain_info)?;
    Ok(reg_chain)
}

type Active = Vec<StakeAddress>;
type Inactive = Vec<StakeAddress>;
pub(crate) fn get_active_inactive_stake_address(
    stake_addresses: HashSet<StakeAddress>,
    cat_id: &CatalystId,
    sqlite: &Sqlite,
    network: CardanoNetwork,
    network_resource: &Network,
) -> anyhow::Result<(Active, Inactive)> {
    let mut active_stake_addresses: Vec<StakeAddress> = Vec::new();
    let mut inactive_stake_addresses: Vec<StakeAddress> = Vec::new();
    for s in stake_addresses {
        let chain_info = select_rbac_registration_chain_from_stake_addr(sqlite, s.clone()).unwrap();
        let reg_chain = build_registration_chain(network, network_resource, chain_info)?;
        // There should be a chain associated with the stake address
        if let Some(r) = reg_chain {
            if r.catalyst_id() == cat_id {
                active_stake_addresses.push(s);
            } else {
                inactive_stake_addresses.push(s);
            }
        } else {
            anyhow::bail!("There should be a chain associated with the stake address {s}");
        }
    }
    Ok((active_stake_addresses, inactive_stake_addresses))
}
