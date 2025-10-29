//! Collects staked TXO and TXI indexed data.
//!
//! Referenced from:
//! - <https://github.com/input-output-hk/catalyst-voices/blob/9bb741c2637bf8fe6814a1dd4e3fe986de536d67/catalyst-gateway/bin/src/db/index/block/txo/mod.rs>
//! - <https://github.com/input-output-hk/catalyst-voices/blob/9bb741c2637bf8fe6814a1dd4e3fe986de536d67/catalyst-gateway/bin/src/db/index/block/txi.rs>

use cardano_blockchain_types::{
    hashes::{Blake2b256Hash, TransactionId},
    pallas_addresses::{Address, ShelleyDelegationPart},
    pallas_traverse::{MultiEraOutput, MultiEraTx},
    MultiEraBlock, Network, StakeAddress,
};
use shared::{
    database::staked_ada::{TxiByTxnIdRow, TxoAssetsByStakeRow, TxoByStakeRow},
    utils::log::{error, warn},
};

/// Temporary buffers, holding entries to be inserted.
#[derive(Default)]
pub struct Buffers {
    pub txo_by_stake: Vec<TxoByStakeRow>,
    pub txo_assets_by_stake: Vec<TxoAssetsByStakeRow>,
    pub txi_by_txn_id: Vec<TxiByTxnIdRow>,
}

impl Buffers {
    /// Index all transaction inputs and outputs in a block.
    pub fn index_block(
        &mut self,
        block: &MultiEraBlock,
    ) {
        for (txn_index, txn) in block.enumerate_txs() {
            let txn_id = TransactionId::from(Blake2b256Hash::from(txn.hash()));
            self.index_txn_txo(
                block.network(),
                &txn,
                block.slot().into(),
                txn_id,
                txn_index.into(),
            );
            self.index_txn_txi(&txn, block.slot().into());
        }
    }

    /// Index the transaction outputs.
    pub fn index_txn_txo(
        &mut self,
        network: Network,
        txn: &MultiEraTx<'_>,
        slot_no: u64,
        txn_id: TransactionId,
        txn_index: u16,
    ) {
        // Accumulate all the data we want to insert from this transaction here.
        for (txo, txo_index) in txn.outputs().iter().zip(0u16..) {
            // This will only return None if the TXO is not to be indexed (Byron Addresses).
            // Skipping missing stake addresses: only indexing staked ADA and assets.
            let Some((Some(stake_address), _)) = extract_stake_address(network, txo, slot_no)
            else {
                continue;
            };

            self.txo_by_stake.push(TxoByStakeRow {
                stake_address: stake_address.clone(),
                txn_id,
                txn_index,
                txo: txo_index,
                slot_no,
                value: txo.value().coin().into(),
                spent_slot: None,
            });

            for asset in txo.value().assets() {
                let policy_id = *asset.policy();
                for policy_asset in asset.assets() {
                    if policy_asset.is_output() {
                        self.txo_assets_by_stake.push(TxoAssetsByStakeRow {
                            stake_address: stake_address.clone(),
                            slot_no,
                            txn_index,
                            txo: txo_index,
                            policy_id,
                            asset_name: policy_asset.name().to_vec(),
                            value: policy_asset.any_coin().into(),
                        });
                    } else {
                        error!("Minting MultiAsset in TXO.");
                    }
                }
            }
        }
    }

    /// Index the transaction inputs.
    pub fn index_txn_txi(
        &mut self,
        txn: &MultiEraTx<'_>,
        slot_no: u64,
    ) {
        for txi in txn.inputs() {
            let txn_id = Blake2b256Hash::from(*txi.hash()).into();
            let txo = txi.index().try_into().unwrap_or(i16::MAX as u16);

            self.txi_by_txn_id.push(TxiByTxnIdRow {
                txn_id,
                txo,
                slot_no,
            });
        }
    }
}

/// Extracts a stake address from a TXO if possible.
/// Returns None if it is not possible.
/// If we want to index, but can not determine a stake key hash, then return a Vec
/// with a single 0 byte.    This is because the index DB needs data in the
/// primary key, so we use a single byte of 0 to indicate    that there is no
/// stake address, and still have a primary key on the table. Otherwise return the
/// header and the stake key hash as a vec of 29 bytes.
fn extract_stake_address(
    network: Network,
    txo: &MultiEraOutput<'_>,
    slot_no: u64,
) -> Option<(Option<StakeAddress>, String)> {
    let stake_address = match txo.address() {
        Ok(address) => {
            match address {
                // Byron addresses do not have stake addresses and are not supported.
                Address::Byron(_) => {
                    return None;
                },
                Address::Shelley(address) => {
                    let address_string = match address.to_bech32() {
                        Ok(address) => address,
                        Err(error) => {
                            // Shouldn't happen, but if it does error and don't index.
                            error!(error:%, slot_no; "Error converting to bech32: skipping.");
                            return None;
                        },
                    };

                    let address = match address.delegation() {
                        ShelleyDelegationPart::Script(hash) => {
                            Some(StakeAddress::new(network, true, (*hash).into()))
                        },
                        ShelleyDelegationPart::Key(hash) => {
                            Some(StakeAddress::new(network, false, (*hash).into()))
                        },
                        ShelleyDelegationPart::Pointer(_pointer) => {
                            // These are not supported from Conway, so we don't support them
                            // either.
                            None
                        },
                        ShelleyDelegationPart::Null => None,
                    };
                    (address, address_string)
                },
                Address::Stake(_) => {
                    // This should NOT appear in a TXO, so report if it does. But don't index it
                    // as a stake address.
                    warn!(slot_no; "Unexpected Stake address found in TXO. Refusing to index.");
                    return None;
                },
            }
        },
        Err(error) => {
            // This should not ever happen.
            error!(error:%, slot_no; "Failed to get Address from TXO. Skipping TXO.");
            return None;
        },
    };

    Some(stake_address)
}
