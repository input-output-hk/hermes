//! Get the RBAC chain by Catalyst ID or stake address.

use catalyst_types::catalyst_id::CatalystId;
use rbac_registration::registration::cardano::RegistrationChain;
use shared::bindings::hermes::{
    cardano::api::{CardanoNetwork, Network},
    sqlite::api::Sqlite,
};

use crate::{
    database::select::cat_id::select_rbac_registration_chain_from_cat_id,
    rbac::{build_rbac_chain::build_registration_chain, rbac_chain_metadata::RbacChainMetadata},
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
