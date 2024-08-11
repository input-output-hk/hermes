//! Metadata decoding and validating.

use std::sync::Arc;

use cip36::Cip36;
use crossbeam_skiplist::SkipMap;
use pallas::ledger::traverse::{MultiEraBlock, MultiEraTx};
use raw_aux_data::RawAuxData;
use tracing::error;

use crate::utils::{i16_from_saturating, usize_from_saturating};

mod cip36;
mod raw_aux_data;

/// List of all validation errors (as strings) Metadata is considered Valid if this list is empty.
pub type ValidationReport = Vec<String>;

/// Possible Decoded Metadata Values.
/// Must match the key they relate too, but the consumer needs to check this.
#[derive(Debug)]
pub enum DecodedMetadataValues {
    // Json Metadata // TODO
    // Json(serde_json::Value), // TODO
    /// CIP-36/CIP-15 Catalyst Registration metadata.
    Cip36(Arc<Cip36>),
}

/// An individual decoded metadata item.
#[derive(Debug)]
pub struct DecodedMetadataItem {
    /// The decoded metadata itself.
    pub value: DecodedMetadataValues,
    /// Validation report for this metadata item.
    pub report: ValidationReport,
}

/// Decoded Metadata for a single transaction.
/// The key is the Primary Label of the Metadata.  
/// For example, CIP15/36 uses labels 61284 & 61285,
/// 61284 is the primary label, so decoded metadata
/// will be under that label.
#[derive(Debug)]
pub(crate) struct DecodedMetadata(SkipMap<u64, Arc<DecodedMetadataItem>>);

impl DecodedMetadata {
    /// Create new decoded metadata for a transaction.
    fn new(slot: u64, txn: &MultiEraTx, raw_aux_data: &RawAuxData) -> Self {
        let decoded_metadata = Self(SkipMap::new());

        // Process each known type of metadata here, and record the decoded result.
        Cip36::decode_and_validate(&decoded_metadata, slot, txn, raw_aux_data, true);

        decoded_metadata
    }

    /// Get the decoded metadata item at the given slot, or None if it doesn't exist.
    pub fn get(&self, primary_label: u64) -> Option<Arc<DecodedMetadataItem>> {
        let entry = self.0.get(&primary_label)?;
        let value = entry.value();
        Some(value.clone())
    }
}

/// Decoded Metadata for a all transactions in a block.
/// The Key for both entries is the Transaction offset in the block.
#[derive(Debug)]
pub struct DecodedTransaction {
    /// The Raw Auxiliary Data for each transaction in the block.
    raw: SkipMap<i16, RawAuxData>,
    /// The Decoded Metadata for each transaction in the block.
    decoded: SkipMap<i16, DecodedMetadata>,
}

impl DecodedTransaction {
    /// Insert another transaction worth of data into the Decoded Aux Data
    fn insert(&mut self, slot: u64, txn_idx: u32, cbor_data: &[u8], transactions: &[MultiEraTx]) {
        let txn_offset = i16_from_saturating(txn_idx);
        let txn_idx = usize_from_saturating(txn_offset);

        let Some(txn) = transactions.get(txn_idx) else {
            error!("No transaction at index {txn_idx} trying to decode metadata.");
            return;
        };

        let txn_raw_aux_data = RawAuxData::new(cbor_data);
        let txn_metadata = DecodedMetadata::new(slot, txn, &txn_raw_aux_data);

        self.raw.insert(txn_offset, txn_raw_aux_data);
        self.decoded.insert(txn_offset, txn_metadata);
    }

    /// Create a new `DecodedTransactionMetadata`.
    pub(crate) fn new(block: &MultiEraBlock) -> Self {
        let mut decoded_aux_data = DecodedTransaction {
            raw: SkipMap::new(),
            decoded: SkipMap::new(),
        };
        let raw_aux_data = SkipMap::new();
        let decoded_metadata = SkipMap::new();

        let transactions = block.txs();
        let slot = block.slot();

        if let Some(_metadata) = block.as_byron() {
            // Nothing to do here.
        } else if let Some(alonzo_block) = block.as_alonzo() {
            for (txn_idx, metadata) in alonzo_block.auxiliary_data_set.iter() {
                decoded_aux_data.insert(slot, *txn_idx, metadata.raw_cbor(), &transactions);
            }
        } else if let Some(babbage_block) = block.as_babbage() {
            for (txn_idx, metadata) in babbage_block.auxiliary_data_set.iter() {
                decoded_aux_data.insert(slot, *txn_idx, metadata.raw_cbor(), &transactions);
            }
        } else if let Some(conway_block) = block.as_conway() {
            for (txn_idx, metadata) in conway_block.auxiliary_data_set.iter() {
                decoded_aux_data.insert(slot, *txn_idx, metadata.raw_cbor(), &transactions);
            }
        } else {
            error!("Undecodable metadata, unknown Era");
        };

        Self {
            raw: raw_aux_data,
            decoded: decoded_metadata,
        }
    }

    /// Get metadata for a given label in a transaction if it exists.
    pub fn get_metadata(&self, txn_idx: i16, label: u64) -> Option<Arc<DecodedMetadataItem>> {
        let txn_metadata = self.decoded.get(&txn_idx)?;
        let txn_metadata = txn_metadata.value();
        let label_metadata = txn_metadata.get(label)?;
        Some(label_metadata)
    }
}
