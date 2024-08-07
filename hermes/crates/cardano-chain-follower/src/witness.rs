//! Transaction Witness
use std::fmt::{Display, Formatter};

use blake2b_simd::{self, Params};
use dashmap::DashMap;
use pallas::codec::utils::Bytes;
use pallas::ledger::traverse::MultiEraTx;

/// `WitnessMap` type of `DashMap` with
/// key as [u8; 28] = (`blake2b_244` hash of the public key)
/// value as (Bytes, Vec<u8>) = (public key, tx index within the block)
#[allow(dead_code)]
pub(crate) type WitnessMap = DashMap<[u8; 28], (Bytes, Vec<u8>)>;

#[derive(Debug)]
#[allow(dead_code)]
/// `TxWitness` struct to store the witness data.
pub(crate) struct TxWitness(WitnessMap);

#[allow(dead_code)]
impl TxWitness {
    /// Create a new `TxWitness` from a list of `MultiEraTx`.
    pub(crate) fn new(txs: &[MultiEraTx]) -> anyhow::Result<Self> {
        let map: WitnessMap = DashMap::new();
        for (i, tx) in txs.iter().enumerate() {
            match tx {
                MultiEraTx::AlonzoCompatible(tx, _) => {
                    let witness_set = &tx.transaction_witness_set;
                    if let Some(vkey_witness_set) = witness_set.vkeywitness.clone() {
                        for vkey_witness in vkey_witness_set {
                            let vkey_hash = blake2b_244(&vkey_witness.vkey)?;
                            let tx_num = u8::try_from(i)?;
                            map.entry(vkey_hash)
                                .and_modify(|entry: &mut (_, Vec<u8>)| entry.1.push(tx_num))
                                .or_insert((vkey_witness.vkey.clone(), vec![tx_num]));
                        }
                    };
                },
                MultiEraTx::Babbage(tx) => {
                    let witness_set = &tx.transaction_witness_set;
                    if let Some(vkey_witness_set) = witness_set.vkeywitness.clone() {
                        for vkey_witness in vkey_witness_set {
                            let vkey_hash = blake2b_244(&vkey_witness.vkey)?;
                            let tx_num = u8::try_from(i)?;
                            map.entry(vkey_hash)
                                .and_modify(|entry: &mut (_, Vec<u8>)| entry.1.push(tx_num))
                                .or_insert((vkey_witness.vkey.clone(), vec![tx_num]));
                        }
                    }
                },
                MultiEraTx::Conway(tx) => {
                    let witness_set = &tx.transaction_witness_set;
                    if let Some(vkey_witness_set) = &witness_set.vkeywitness.clone() {
                        for vkey_witness in vkey_witness_set {
                            let vkey_hash = blake2b_244(&vkey_witness.vkey)?;
                            let tx_num = u8::try_from(i)?;
                            map.entry(vkey_hash)
                                .and_modify(|entry: &mut (_, Vec<u8>)| entry.1.push(tx_num))
                                .or_insert((vkey_witness.vkey.clone(), vec![tx_num]));
                        }
                    }
                },
                _ => {},
            };
        }
        Ok(Self(map))
    }

    /// Check whether the public key hash is in the given transaction number.
    pub(crate) fn check_witness_in_tx(&self, vkey: &[u8; 28], tx_num: u8) -> bool {
        self.0
            .get(vkey)
            .map_or(false, |entry| entry.1.contains(&tx_num))
    }

    /// Get the actual address from the given public key hash.
    pub(crate) fn get_witness_addr(&self, vkey: &[u8; 28]) -> Option<Bytes> {
        self.0.get(vkey).map(|entry| entry.0.clone())
    }
}

impl Display for TxWitness {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        for data in &self.0 {
            let vkey_hash = hex::encode(data.key());
            let vkey: Vec<u8> = data.0.clone().into();
            let vkey_encoded = hex::encode(&vkey);
            writeln!(
                f,
                "Key Hash: {}, PublicKey: {}, Tx: {:?}",
                vkey_hash, vkey_encoded, data.1
            )?;
        }
        Ok(())
    }
}

/// Convert the given value to `blake2b_244` array.
pub(crate) fn blake2b_244(value: &[u8]) -> anyhow::Result<[u8; 28]> {
    let h = Params::new().hash_length(28).hash(value);
    let b = h.as_bytes();
    b.try_into()
        .map_err(|_| anyhow::anyhow!("Invalid length of blake2b_244, expected 28 got {}", b.len()))
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::multi_era_block_data::tests::alonzo_block;

    #[test]
    fn tx_witness() {
        let alonzo = alonzo_block();
        let alonzo_block = pallas::ledger::traverse::MultiEraBlock::decode(&alonzo)
            .expect("Fail to decode MultiEraBlock");
        let txs = alonzo_block.txs();
        let tx_witness = TxWitness::new(&txs).expect("Fail to create TxWitness");
        let vkey1: [u8; 28] =
            hex::decode("6082eb618d161a704207a0b3a9609e820111570d94d1e711b005386c")
                .expect("Fail to decode vkey1")
                .try_into()
                .expect("Invalid length of vkey1");
        println!("{tx_witness}");
        assert!(tx_witness.get_witness_addr(&vkey1).is_some());
        assert!(tx_witness.check_witness_in_tx(&vkey1, 0));
    }
}
