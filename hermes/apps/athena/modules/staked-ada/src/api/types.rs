//! Staked ada helper types.

use std::collections::HashMap;

use cardano_blockchain_types::{Slot, TxnIndex, hashes::TransactionId};
use shared::utils::common::{
    objects::cardano::stake_info::FullStakeInfo, responses::WithErrorResponses,
};

/// Endpoint responses.
pub(crate) enum Responses {
    /// ## Ok
    ///
    /// The amount of ADA staked by the queried stake address, as at the indicated slot.
    Ok(FullStakeInfo),
    /// ## Not Found
    ///
    /// The queried stake address was not found at the requested slot number.
    NotFound,
}

/// All responses.
pub(crate) type AllResponses = WithErrorResponses<Responses>;

/// TXO information used when calculating a user's stake info.
#[derive(Clone)]
pub(crate) struct TxoInfo {
    /// TXO value.
    pub(crate) value: num_bigint::BigInt,
    /// TXO transaction index within the slot.
    pub(crate) txn_index: TxnIndex,
    /// TXO index.
    pub(crate) txo: u16,
    /// TXO transaction slot number.
    pub(crate) slot_no: Slot,
    /// Whether the TXO was spent.
    pub(crate) spent_slot_no: Option<Slot>,
}

/// `TxoInfo` map type alias
pub(crate) type TxoMap = HashMap<(TransactionId, u16), TxoInfo>;

/// TXO Assets map type alias
pub(crate) type TxoAssetsMap =
    HashMap<GetAssetsByStakeAddressQueryKey, Vec<GetAssetsByStakeAddressQueryValue>>;

/// TXO Assets state
#[derive(Default, Clone)]
pub(crate) struct TxoAssetsState {
    /// TXO Info map
    pub(crate) txos: TxoMap,
    /// TXO Assets map
    pub(crate) txo_assets: TxoAssetsMap,
}

impl TxoAssetsState {
    /// Returns true if underlying `txos` and `txo_assets` are empty, false otherwise
    pub(crate) fn is_empty(&self) -> bool {
        self.txos.is_empty() && self.txo_assets.is_empty()
    }
}

/// Get native assets query key
#[derive(Hash, PartialEq, Eq, Debug, Clone)]
pub(crate) struct GetAssetsByStakeAddressQueryKey {
    /// TXO transaction index within the slot.
    pub txn_index: u16,
    /// TXO index.
    pub txo: u16,
    /// TXO transaction slot number.
    pub slot_no: u64,
}

/// Get native assets query value
#[derive(Clone)]
pub(crate) struct GetAssetsByStakeAddressQueryValue {
    /// Asset policy hash (28 bytes).
    pub policy_id: Vec<u8>,
    /// Asset name (range of 0 - 32 bytes)
    pub asset_name: Vec<u8>,
    /// Asset value.
    pub value: num_bigint::BigInt,
}
