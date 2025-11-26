//! Build the RBAC registration chain

use rbac_registration::{cardano::cip509::Cip509, registration::cardano::RegistrationChain};
use shared::{
    bindings::hermes::cardano::api::{CardanoNetwork, Network},
    utils::{cardano::block::build_block, log::log_error},
};

use crate::rbac::registration_location::RegistrationLocation;

/// Build the RBAC registration chain.
///
/// # Arguments
///
/// * `network` - The network to build the registration chain.
/// * `network_resource` - The network resource used for getting block data.
/// * `registration_location` - The registration chain information.
///
/// # Return
///
/// * `Ok(Option<RegistrationChain>)` – A RBAC registration chain or `None` if
///   registration chain is empty.
/// * `Err(anyhow::Error)` - If any error occurs.
pub(crate) fn build_registration_chain(
    network: CardanoNetwork,
    network_resource: &Network,
    registration_location: &[RegistrationLocation],
) -> anyhow::Result<Option<RegistrationChain>> {
    const FUNCTION_NAME: &str = "build_registration_chain";

    // The first registration (root)
    let Some(first_info) = registration_location.first() else {
        return Ok(None);
    };

    // Root registration use to initialize chain
    let root_reg = get_registration(
        FUNCTION_NAME,
        network,
        network_resource,
        first_info.slot_no,
        first_info.txn_idx,
    )?;
    let mut reg_chain = RegistrationChain::new_stateless(&root_reg).ok_or_else(|| {
        let error = "Failed to initialize registration chain";
        log_error(
            file!(),
            FUNCTION_NAME,
            "RegistrationChain::new",
            error,
            None,
        );
        anyhow::anyhow!(error)
    })?;

    // Append children
    for info in registration_location.iter().skip(1) {
        let reg = get_registration(
            file!(),
            network,
            network_resource,
            info.slot_no,
            info.txn_idx,
        )?;
        if let Some(updated) = reg_chain.update_stateless(&reg) {
            // Broken registration in the chain doesn't break the early created chain,
            // there is no need to continue the chain since the data after a broken
            // registration should be ignored
            if reg.report().is_problematic() {
                return Ok(Some(reg_chain));
            }
            // If the registration being update is not problematic
            // It can be added to the registration chain
            reg_chain = updated;
        }
    }
    Ok(Some(reg_chain))
}

/// Get a RBAC registration (CIP509) from a block.
fn get_registration(
    func_name: &str,
    network: CardanoNetwork,
    network_resource: &Network,
    slot_no: u64,
    txn_idx: u16,
) -> anyhow::Result<Cip509> {
    let block_resource = network_resource
        .get_block(Some(slot_no), 0)
        .ok_or_else(|| {
            let err = format!("Failed to get block resource at slot {slot_no}");
            log_error(file!(), func_name, "network.get_block", &err, None);
            anyhow::anyhow!(err)
        })?;

    // Create a multi-era block
    let block = build_block(file!(), func_name, network, &block_resource).ok_or_else(|| {
        let err = format!("Failed to build block at slot {slot_no}");
        log_error(file!(), func_name, "build_block", &err, None);
        anyhow::anyhow!(err)
    })?;

    if let Ok(Some(r)) = Cip509::new(&block, txn_idx.into(), &[]) {
        Ok(r)
    } else {
        let err = format!("Failed to get registration at slot {slot_no}");
        log_error(file!(), func_name, "Cip509::new", &err, None);
        anyhow::bail!(err)
    }
}
