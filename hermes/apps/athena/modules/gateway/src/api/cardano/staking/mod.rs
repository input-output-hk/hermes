//! Cardano Staking API Endpoints.

use serde::{Deserialize, Serialize};

use crate::common::{
    auth::none_or_rbac::NoneOrRBAC,
    objects::cardano::network::Network,
    types::cardano::{self, cip19_stake_address::Cip19StakeAddress, slot_no::SlotNo},
};

mod assets_get;
mod db_mocked;

/// Cardano Staking API Endpoints
pub(crate) struct Api;

#[derive(Serialize, Deserialize)]
pub(crate) struct GetStakedAdaRequest {
    // Cardano network type.
    // If omitted network type is identified from the stake address.
    // If specified it must be correspondent to the network type encoded in the stake
    // address.
    // As `preprod` and `preview` network types in the stake address encoded as a
    // `testnet`, to specify `preprod` or `preview` network type use this
    // query parameter.
    // network: Query<Option<Network>>,
    pub(crate) network: Option<Network>,
    // A time point at which the assets should be calculated.
    // If omitted latest slot number is used.
    // asat: Query<Option<cardano::query::AsAt>>,
    pub(crate) asat: Option<cardano::as_at::AsAt>,
    // No Authorization required, but Token permitted.
    // pub(crate) auth: NoneOrRBAC,
}

// #[OpenApi(tag = "ApiTags::Cardano")]
impl Api {
    /// Get staked assets.
    ///
    /// This endpoint returns the total Cardano's staked assets to the corresponded
    /// user's stake address.
    // #[oai(
    //     path = "/v1/cardano/assets/:stake_address",
    //     method = "get",
    //     operation_id = "stakedAssetsGet"
    // )]
    pub fn staked_ada_get(
        &self,
        // The stake address of the user.
        // Should be a valid Bech32 encoded address followed by the https://cips.cardano.org/cip/CIP-19/#stake-addresses.
        // stake_address: Path<Cip19StakeAddress>,
        stake_address: Cip19StakeAddress,
        // Cardano network type.
        // If omitted network type is identified from the stake address.
        // If specified it must be correspondent to the network type encoded in the stake
        // address.
        // As `preprod` and `preview` network types in the stake address encoded as a
        // `testnet`, to specify `preprod` or `preview` network type use this
        // query parameter.
        // network: Query<Option<Network>>,
        network: Option<Network>,
        // A time point at which the assets should be calculated.
        // If omitted latest slot number is used.
        // asat: Query<Option<cardano::query::AsAt>>,
        asat: Option<cardano::as_at::AsAt>,
        // No Authorization required, but Token permitted.
        _auth: NoneOrRBAC,
    ) -> assets_get::AllResponses {
        assets_get::endpoint(stake_address, network, SlotNo::into_option(asat))
    }
}
