//! Get the RBAC chain by Catalyst ID or stake address.

use cardano_blockchain_types::StakeAddress;
use rbac_registration::registration::cardano::RegistrationChain;

use crate::{
    database::select::{
        select_cat_id::select_rbac_registration_chain_from_cat_id,
        select_stake::select_rbac_registration_chain_from_stake_addr,
    },
    hermes::{
        self,
        hermes::{cardano::api::Network, sqlite::api::Sqlite},
    },
    rbac::build_rbac_chain::build_registration_chain,
};

/// Get the RBAC chain by Catalyst ID.
pub(crate) fn get_rbac_chain(
    sqlite: &Sqlite,
    cat_id: &str,
    network: hermes::hermes::cardano::api::CardanoNetwork,
    network_resource: &Network,
) -> anyhow::Result<Option<RegistrationChain>> {
    let chain_info = select_rbac_registration_chain_from_cat_id(sqlite, cat_id, network)?;
    let reg_chain = build_registration_chain(network, network_resource, chain_info)?;
    Ok(reg_chain)
}

/// Get the RBAC chain by Stake address.
pub(crate) fn get_rbac_chain_from_stake_address(
    sqlite: &Sqlite,
    stake_address: StakeAddress,
    network: hermes::hermes::cardano::api::CardanoNetwork,
    network_resource: &Network,
) -> anyhow::Result<Option<RegistrationChain>> {
    let chain_info =
        select_rbac_registration_chain_from_stake_addr(sqlite, stake_address, network)?;
    let reg_chain = build_registration_chain(network, network_resource, chain_info)?;
    Ok(reg_chain)
}
