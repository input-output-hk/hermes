//! Defines API schemas of stake amount type.

use derive_more::{From, Into};

use crate::common::types::{
    array_types::impl_array_types,
    cardano::{
        ada_value::AdaValue, asset_name::AssetName, asset_value::AssetValue,
        hash28::HexEncodedHash28, slot_no::SlotNo,
    },
};

/// User's staked txo asset info.
// #[derive(Object, Debug, Clone)]
#[derive(Debug, Clone)]
// #[oai(example)]
pub(crate) struct StakedTxoAssetInfo {
    /// Asset policy hash (28 bytes).
    pub(crate) policy_hash: HexEncodedHash28,
    /// Token policies Asset Name.
    pub(crate) asset_name: AssetName,
    /// Token Asset Value.
    pub(crate) amount: AssetValue,
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
pub(crate) struct StakeInfo {
    /// Total stake amount.
    pub(crate) ada_amount: AdaValue,

    /// Block's slot number which contains the latest unspent UTXO.
    pub(crate) slot_number: SlotNo,

    /// TXO assets infos.
    pub(crate) assets: StakedAssetInfoList,
}

/// Volatile stake information.
#[derive(From, Into)]
pub(crate) struct VolatileStakeInfo(StakeInfo);

/// Persistent stake information.
#[derive(From, Into)]
pub(crate) struct PersistentStakeInfo(StakeInfo);

/// Full user's cardano stake info.
pub(crate) struct FullStakeInfo {
    /// Volatile stake information.
    pub(crate) volatile: VolatileStakeInfo,
    /// Persistent stake information.
    pub(crate) persistent: PersistentStakeInfo,
}
