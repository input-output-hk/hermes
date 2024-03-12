//! Implementation of Bip32-Ed25519.

use crate::runtime_extensions::bindings::hermes::{
    binary::api::Bstr,
    crypto::api::{Bip32Ed25519PublicKey, Bip32Ed25519Signature},
};
use bip32::DerivationPath;
use anyhow::Error;
use ed25519_bip32::{DerivationScheme, Signature, XPrv};

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
pub(crate) fn derive_new_private_key(private_key: XPrv, path: &str) -> Result<XPrv, Error> {
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
            true => xprv.derive(DerivationScheme::V2, child_num.index() | 0x80_00_00_00),
            false => xprv.derive(DerivationScheme::V2, child_num.index()),
        }
    });

    Ok(key)
}

// FIXME - Better implementation
fn array_u8_32_to_tuple(array: &[u8; 32]) -> (u64, u64, u64, u64) {
    let mut tuple = (0u64, 0u64, 0u64, 0u64);
    let mut arr = [0u8; 8];
    let slice1 = &array[0..8];
    arr.copy_from_slice(&slice1);
    tuple.0 = u64::from_be_bytes(arr);

    let slice2 = &array[8..16];
    arr.copy_from_slice(&slice2);
    tuple.1 = u64::from_be_bytes(arr);

    let slice3 = &array[16..24];
    arr.copy_from_slice(&slice3);
    tuple.2 = u64::from_be_bytes(arr);

    let slice4 = &array[24..32];
    arr.copy_from_slice(&slice4);
    tuple.3 = u64::from_be_bytes(arr);

    tuple
}

// FIXME - Better implementation
fn array_u8_64_to_tuple(array: &[u8; 64]) -> (u64, u64, u64, u64, u64, u64, u64, u64) {
    // Extract eight u64 values from the [u8; 32] array
    let mut tuple = (0u64, 0u64, 0u64, 0u64, 0u64, 0u64, 0u64, 0u64);
    let mut arr = [0u8; 8];
    let slice1 = &array[0..8];
    arr.copy_from_slice(&slice1);
    tuple.0 = u64::from_be_bytes(arr);

    let slice2 = &array[8..16];
    arr.copy_from_slice(&slice2);
    tuple.1 = u64::from_be_bytes(arr);

    let slice3 = &array[16..24];
    arr.copy_from_slice(&slice3);
    tuple.2 = u64::from_be_bytes(arr);

    let slice4 = &array[24..32];
    arr.copy_from_slice(&slice4);
    tuple.3 = u64::from_be_bytes(arr);

    let slice5 = &array[32..40];
    arr.copy_from_slice(&slice5);
    tuple.4 = u64::from_be_bytes(arr);

    let slice6 = &array[40..48];
    arr.copy_from_slice(&slice6);
    tuple.5 = u64::from_be_bytes(arr);

    let slice7 = &array[48..56];
    arr.copy_from_slice(&slice7);
    tuple.6 = u64::from_be_bytes(arr);

    let slice8 = &array[56..64];
    arr.copy_from_slice(&slice8);
    tuple.7 = u64::from_be_bytes(arr);

    tuple
}

// FIXME - Better implementation
fn b512_u64_tuple_to_u8_array(tuple: &(u64, u64, u64, u64, u64, u64, u64, u64)) -> [u8; 64] {
    let mut bytes = [0u8; 64];
    let (a, b, c, d, e, f, g, h) = tuple;
    bytes[0..8].copy_from_slice(&a.to_be_bytes());
    bytes[8..16].copy_from_slice(&b.to_be_bytes());
    bytes[16..24].copy_from_slice(&c.to_be_bytes());
    bytes[24..32].copy_from_slice(&d.to_be_bytes());
    bytes[32..40].copy_from_slice(&e.to_be_bytes());
    bytes[40..48].copy_from_slice(&f.to_be_bytes());
    bytes[48..56].copy_from_slice(&g.to_be_bytes());
    bytes[56..64].copy_from_slice(&h.to_be_bytes());
    bytes
}

#[cfg(test)]
mod tests_bip32_ed25519 {
    use super::*;

    // Test vectors are converted from SLIP-0010.
    // https://github.com/satoshilabs/slips/blob/master/slip-0010.md#test-vector-1-for-ed25519
    const XPRV1: [u8; 32] = [
        43, 75, 231, 241, 158, 226, 123, 191, 48, 198, 103, 182, 66, 213, 244, 170, 105, 253, 22,
        152, 114, 248, 252, 48, 89, 192, 142, 186, 226, 235, 25, 231,
    ];
    const CHAINCODE1: [u8; 32] = [
        144, 4, 106, 147, 222, 83, 128, 167, 43, 94, 69, 1, 7, 72, 86, 125, 94, 160, 43, 191, 101,
        34, 249, 121, 224, 92, 13, 141, 140, 169, 255, 251,
    ];
    const PUBKEY1: &str = "00a4b2856bfec510abab89753fac1ac0e1112364e7d250545963f135f2a33188ed";
    const DATA: &[u8; 4] = b"test";

    #[test]
    fn test_get_public_key() {
        let xprv = XPrv::from_nonextended_force(&XPRV1, &CHAINCODE1);
        let pubk_tuple = get_public_key(xprv);
        let pubk_hex = format!(
            "{:x}{:x}{:x}{:x}",
            pubk_tuple.0, pubk_tuple.1, pubk_tuple.2, pubk_tuple.3
        );
        assert_eq!(pubk_hex, PUBKEY1);
    }
    #[test]
    fn test_sign_data_and_check_signature() {
        let xprv = XPrv::from_nonextended_force(&XPRV1, &CHAINCODE1);
        println!("{:?}", xprv);

        let sign_data = sign_data(xprv.clone(), DATA);
        let check_signature = check_signature(xprv, DATA, sign_data);
        assert_eq!(check_signature, true);
    }

    #[test]
    fn test_derive_new_private_key() {
        let xprv = XPrv::from_nonextended_force(&XPRV1, &CHAINCODE1);
        let derived_xprv = derive_new_private_key(xprv, "m/0'").unwrap();
        println!("{:?}", derived_xprv);
    }
}
