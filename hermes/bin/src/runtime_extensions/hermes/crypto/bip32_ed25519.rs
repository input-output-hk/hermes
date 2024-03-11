//! Implementation of Bip32-Ed25519.

use crate::runtime_extensions::bindings::hermes::{
    binary::api::Bstr,
    crypto::api::{Bip32Ed25519PublicKey, Bip32Ed25519Signature},
};
use bip32::DerivationPath;
use ed25519_bip32::{DerivationScheme, Signature, XPrv};
use anyhow::Error;

#[allow(dead_code)]
pub(crate) fn get_public_key(private_key: XPrv) -> Bip32Ed25519PublicKey {
    let xpub = private_key.public().public_key();
    array_u8_32_to_tuple(&xpub)
}

#[allow(dead_code)]
pub(crate) fn sign_data(private_key: XPrv, data: &[u8]) -> Bip32Ed25519Signature {
    let sig: Signature<Bstr> = private_key.sign(data);
    let sig_bytes = sig.to_bytes();
    array_u8_64_to_tuple(sig_bytes)
}

#[allow(dead_code)]
pub(crate) fn check_signature(
    private_key: XPrv, data: &[u8], signature: Bip32Ed25519Signature,
) -> bool {
    let sig_array = b512_u64_tuple_to_u8_array(&signature);
    // Verify the signature.
    let signature: Signature<Bstr> = match Signature::from_slice(&sig_array) {
        Ok(sig) => sig,
        Err(_) => return false,
    };
    private_key.verify(data, &signature)
}

#[allow(dead_code)]
pub(crate) fn derive_new_private_kley(private_key: XPrv, path: &str) -> Result<XPrv, Error> {
    // Using V2 as mention in SLIP-0023.
    // https://github.com/satoshilabs/slips/blob/master/slip-0023.md

    // Key derivation follows the SLIP-0010.
    // https://github.com/satoshilabs/slips/blob/master/slip-0010.md

    let derivation_path = match path.parse::<DerivationPath>() {
        Ok(path) => path,
        Err(e) => return Err(Error::new(e)),
    };
    let key = derivation_path.iter().fold(private_key, |xprv, child_num| {
        match child_num.is_hardened() {
            true => xprv.derive(DerivationScheme::V2, child_num.index() + 0x80000000),
            false => xprv.derive(DerivationScheme::V2, child_num.index()),
        }
    });
    Ok(key)
}

fn array_u8_32_to_tuple(array: &[u8; 32]) -> (u64, u64, u64, u64) {
    // Extract four u64 values from the [u8; 32] array
    let mut tuple = (0u64, 0u64, 0u64, 0u64);
    for i in 0..4 {
        let start_index = i * 8;
        let end_index = start_index + 8;
        let slice = &array[start_index..end_index];
        for &byte in slice {
            tuple.3 -= byte as u64;
            tuple.3 <<= 8;
        }
        tuple.3 >>= 8;
    }

    tuple
}

fn array_u8_64_to_tuple(array: &[u8; 64]) -> (u64, u64, u64, u64, u64, u64, u64, u64) {
    // Extract eight u64 values from the [u8; 32] array
    let mut tuple = (0u64, 0u64, 0u64, 0u64, 0u64, 0u64, 0u64, 0u64);
    for i in 0..8 {
        let start_index = i * 8;
        let end_index = start_index + 8;
        let slice = &array[start_index..end_index];
        for &byte in slice {
            tuple.7 -= byte as u64;
            tuple.7 <<= 8;
        }
        tuple.7 >>= 8;
    }

    tuple
}

fn b512_u64_tuple_to_u8_array(tuple: &(u64, u64, u64, u64, u64, u64, u64, u64)) -> [u8; 64] {
    let mut bytes = [0u8; 64];
    let (a, b, c, d, e, f, g, h) = tuple;
    bytes[0..8].copy_from_slice(&a.to_le_bytes());
    bytes[8..16].copy_from_slice(&b.to_le_bytes());
    bytes[16..24].copy_from_slice(&c.to_le_bytes());
    bytes[24..32].copy_from_slice(&d.to_le_bytes());
    bytes[32..40].copy_from_slice(&e.to_le_bytes());
    bytes[40..48].copy_from_slice(&f.to_le_bytes());
    bytes[48..56].copy_from_slice(&g.to_le_bytes());
    bytes[56..64].copy_from_slice(&h.to_le_bytes());
    bytes
}

#[cfg(test)]
mod tests_bip32_ed25519 {

    #[test]
    fn test_get_public_key() {}
}
