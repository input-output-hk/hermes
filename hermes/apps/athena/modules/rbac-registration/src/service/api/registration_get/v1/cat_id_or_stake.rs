//! Catalyst ID or stake address.

use catalyst_types::catalyst_id::CatalystId;

use crate::service::common::types::cardano::cip19_stake_address::Cip19StakeAddress;

/// A CIP-19 stake address, or a Catalyst Id
#[derive(Debug, Clone)]
pub(crate) enum CatIdOrStake {
    /// A CIP-19 stake address
    Address(Cip19StakeAddress),
    /// A catalyst id
    CatId(CatalystId),
}

impl TryFrom<&str> for CatIdOrStake {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> std::result::Result<Self, Self::Error> {
        match value.parse::<CatalystId>() {
            Ok(cat_id) => Ok(Self::CatId(cat_id)),
            Err(_) => {
                match Cip19StakeAddress::try_from(value) {
                    Ok(stake_addr) => Ok(Self::Address(stake_addr)),
                    Err(_) => {
                        anyhow::bail!(
                            "Not a valid \"Catalyst Id or Stake Address\" parameter given {value}."
                        )
                    },
                }
            },
        }
    }
}
