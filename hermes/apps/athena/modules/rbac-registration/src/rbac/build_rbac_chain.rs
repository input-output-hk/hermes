//! Build the RBAC chain

use anyhow::bail;
use cardano_blockchain_types::Point;
use rbac_registration::{cardano::cip509::Cip509, registration::cardano::RegistrationChain};

use crate::{
    hermes::hermes::cardano::{
        self,
        api::{CardanoNetwork, Network},
    },
    utils::{cardano::block::build_block, log::log_error},
};

/// Information needed to build the RBAC chain
#[derive(Debug, Clone)]
pub(crate) struct RbacChainInfo {
    /// The slot number of the block that contain the registration.
    pub(crate) slot_no: u64,
    /// The transaction index that contain the registration.
    pub(crate) txn_idx: u16,
}

/// Build the RBAC registration chain.
///
/// # Return
///
/// * `Ok(Option<RegistrationChain>)` – A RBAC registration chain
///   or `None` if registration chain is empty.
/// * `Err(anyhow::Error)` - If any error occurs.

pub(crate) fn build_registration_chain(
    network: CardanoNetwork,
    network_resource: &Network,
    rbac_chain_info: Vec<RbacChainInfo>,
) -> anyhow::Result<Option<RegistrationChain>> {
    const FUNCTION_NAME: &str = "build_registration_chain";

    // The first registration (root)
    let first_info = match rbac_chain_info.first() {
        Some(info) => info,
        None => {
            log_error(
                file!(),
                FUNCTION_NAME,
                "rbac_chain_info.first",
                "Registration chain info is empty",
                None,
            );
            return Ok(None);
        },
    };

    let block_resource = match network_resource.get_block(Some(first_info.slot_no), 0) {
        Some(br) => br,
        None => {
            log_error(
                file!(),
                FUNCTION_NAME,
                "network.get_block",
                &format!(
                    "Failed to get block resource at slot {}",
                    first_info.slot_no
                ),
                None,
            );
            return bail!("Failed to get block resource");
        },
    };

    let block = match build_block(file!(), FUNCTION_NAME, network, &block_resource) {
        Some(b) => b,
        None => {
            log_error(
                file!(),
                FUNCTION_NAME,
                "build_block",
                &format!("Failed to build block at slot {}", first_info.slot_no),
                None,
            );
            return bail!("Failed to build block");
        },
    };

    let root_reg = match Cip509::new(&block, first_info.txn_idx.into(), &[]) {
        Ok(Some(r)) => r,
        Ok(None) | Err(_) => {
            log_error(
                file!(),
                FUNCTION_NAME,
                "Cip509::new",
                &format!(
                    "Failed to create root registration at slot {}",
                    first_info.slot_no
                ),
                None,
            );
            return bail!("Failed to create registration");
        },
    };

    let mut reg_chain = match RegistrationChain::new(root_reg) {
        Some(chain) => chain,
        None => {
            log_error(
                file!(),
                FUNCTION_NAME,
                "RegistrationChain::new",
                "Failed to initialize registration chain",
                None,
            );
            return bail!("Failed to initialize registration chain");
        },
    };

    for info in rbac_chain_info.iter().skip(1) {
        let block_resource = match network_resource.get_block(Some(info.slot_no), 0) {
            Some(br) => br,
            None => {
                log_error(
                    file!(),
                    FUNCTION_NAME,
                    "network.get_block",
                    &format!("Failed to get block resource at slot {}", info.slot_no),
                    None,
                );
                return bail!("Failed to get block resource");
            },
        };

        let block = match build_block(file!(), FUNCTION_NAME, network, &block_resource) {
            Some(b) => b,
            None => {
                log_error(
                    file!(),
                    FUNCTION_NAME,
                    "build_block",
                    &format!("Failed to build block at slot {}", info.slot_no),
                    None,
                );
                return bail!("Failed to build block");
            },
        };

        let reg = match Cip509::new(&block, info.txn_idx.into(), &[]) {
            Ok(Some(r)) => r,
            Ok(None) | Err(_) => {
                log_error(
                    file!(),
                    FUNCTION_NAME,
                    "Cip509::new",
                    &format!("Failed to create registration at slot {}", info.slot_no),
                    None,
                );
                return bail!("Failed to create registration");
            },
        };

        reg_chain = reg_chain.update(reg).unwrap();
    }
    Ok(Some(reg_chain))
}
