//! Defines API schemas of stake amount type.

use derive_more::{From, Into};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::utils::common::types::{
    array_types::impl_array_types,
    cardano::{
        ada_value::AdaValue, asset_name::AssetName, asset_value::AssetValue,
        hash28::HexEncodedHash28, slot_no::SlotNo,
    },
};

/// User's staked txo asset info.
// #[derive(Object, Debug, Clone)]
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
// #[oai(example)]
pub struct StakedTxoAssetInfo {
    /// Asset policy hash (28 bytes).
    pub policy_hash: HexEncodedHash28,
    /// Token policies Asset Name.
    pub asset_name: AssetName,
    /// Token Asset Value.
    pub amount: AssetValue,
}

// List of User's Staked Native Token Info
impl_array_types!(
    StakedAssetInfoList,
    StakedTxoAssetInfo,
    Some(poem_openapi::registry::MetaSchema {
        example: Self::example().to_json(),
        max_items: Some(1000),
        items: Some(Box::new(StakedTxoAssetInfo::schema_ref())),
        ..poem_openapi::registry::MetaSchema::ANY
    })
);

/// User's cardano stake info.
#[derive(Serialize, Deserialize, ToSchema)]
pub struct StakeInfo {
    /// Total stake amount.
    pub ada_amount: AdaValue,

    /// Block's slot number which contains the latest unspent UTXO.
    pub slot_number: SlotNo,

    /// TXO assets infos.
    pub assets: StakedAssetInfoList,
}

/// Volatile stake information.
#[derive(From, Into, Serialize, Deserialize, ToSchema)]
pub struct VolatileStakeInfo(StakeInfo);

/// Persistent stake information.
#[derive(From, Into, Serialize, Deserialize, ToSchema)]
pub struct PersistentStakeInfo(StakeInfo);

/// Full user's cardano stake info.
#[derive(Serialize, Deserialize, ToSchema)]
pub struct FullStakeInfo {
    /// Volatile stake information.
    pub volatile: VolatileStakeInfo,
    /// Persistent stake information.
    pub persistent: PersistentStakeInfo,
}
