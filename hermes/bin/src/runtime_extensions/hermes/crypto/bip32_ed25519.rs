//! Implementation of Bip32-Ed25519.

use crate::runtime_extensions::bindings::hermes::{
    binary::api::Bstr,
    crypto::api::{Bip32Ed25519PublicKey, Bip32Ed25519Signature, Errno},
};
use bip32::DerivationPath;
use ed25519_bip32::{DerivationScheme, Signature, XPrv};

/// Get public key from the given extended private key.
///
/// # Arguments
///
/// - `xprivate_key`: An extended private key of type `XPrv`.
///
/// # Returns
///
/// Returns a tuple of u64 values with length 4 representing the public key.
pub(crate) fn get_public_key(xprivate_key: &XPrv) -> Bip32Ed25519PublicKey {
    let xpub = xprivate_key.public().public_key();
    array_u8_32_to_tuple(&xpub)
}

/// Sign data with the given extended private key.
///
/// # Arguments
///
/// - `xprivate_key`: An extended private key of type `XPrv`.
/// - `data`: The data to sign.
///
/// # Returns
/// Returns a tuple of u64 values with length 8 representing the signature.
pub(crate) fn sign_data(xprivate_key: &XPrv, data: &Bstr) -> Bip32Ed25519Signature {
    let sig: Signature<Bstr> = xprivate_key.sign(data);
    let sig_bytes = sig.to_bytes();
    array_u8_64_to_tuple(sig_bytes)
}

/// Check the signature on the given data.
///
/// # Arguments
///
/// - `xprivate_key`: An extended private key of type `XPrv`.
/// - `data`: The data to sign.
/// - `signature`: The signature to check.
///
/// # Returns
/// Returns a boolean value indicating if the signature match the sign data
/// from `xprivate_key` and data.
/// True if the signature is valid and match the sign data, false otherwise.
pub(crate) fn check_signature(
    xprivate_key: &XPrv, data: &Bstr, signature: Bip32Ed25519Signature,
) -> bool {
    let sig_array = b512_u64_tuple_to_u8_array(&signature);
    // Verify the signature.
    let signature: Signature<Bstr> = match Signature::from_slice(&sig_array) {
        Ok(sig) => sig,
        // Invalid signature
        Err(_) => return false,
    };
    xprivate_key.verify(data, &signature)
}

/// Derive a new extended private key from the given extended private key.
/// - V2 derivation scheme is used as it is mention in
/// [SLIP-0023](https://github.com/satoshilabs/slips/blob/master/slip-0023.md).
/// - More information about child key derivation can be found in
/// [BIP32-Ed25519](https://input-output-hk.github.io/adrestia/static/Ed25519_BIP.pdf).
///  
/// # Arguments
///
/// - `xprivate_key`: An extended private key of type `XPrv`.
/// - `path`: Derivation path. eg. m/0/2'/3 where ' represents hardened derivation.
///
/// # Returns
///
/// Returns the `XPrv` extended private key as a `Result`.
/// If the derivation path is successful, it reutrns `Ok` with the extended private key (`XPrv`).
///
/// # Errors
///
/// Returns an `InvalidDerivationalPath` if the derivation path is invalid.
pub(crate) fn derive_new_private_key(xprivate_key: XPrv, path: &str) -> Result<XPrv, Errno> {
    let Ok(derivation_path) = path.parse::<DerivationPath>() else {
        return Err(Errno::InvalidDerivationalPath);
    };
    let key = derivation_path
        .iter()
        .fold(xprivate_key, |xprv, child_num| {
            if child_num.is_hardened() {
                xprv.derive(DerivationScheme::V2, child_num.index() | 0x80_00_00_00)
            } else {
                xprv.derive(DerivationScheme::V2, child_num.index())
            }
        });
    Ok(key)
}

/// Convert a 32 bytes array to a tuple of u64 values.
fn array_u8_32_to_tuple(array: &[u8; 32]) -> (u64, u64, u64, u64) {
    let mut tuple = (0u64, 0u64, 0u64, 0u64);
    let mut arr = [0u8; 8];
    let slice1 = &array[0..8];
    arr.copy_from_slice(slice1);
    tuple.0 = u64::from_be_bytes(arr);

    let slice2 = &array[8..16];
    arr.copy_from_slice(slice2);
    tuple.1 = u64::from_be_bytes(arr);

    let slice3 = &array[16..24];
    arr.copy_from_slice(slice3);
    tuple.2 = u64::from_be_bytes(arr);

    let slice4 = &array[24..32];
    arr.copy_from_slice(slice4);
    tuple.3 = u64::from_be_bytes(arr);

    tuple
}

/// Convert a 64 bytes array to a tuple of u64 values.
fn array_u8_64_to_tuple(array: &[u8; 64]) -> (u64, u64, u64, u64, u64, u64, u64, u64) {
    let mut tuple = (0u64, 0u64, 0u64, 0u64, 0u64, 0u64, 0u64, 0u64);
    let mut arr = [0u8; 8];
    let slice1 = &array[0..8];
    arr.copy_from_slice(slice1);
    tuple.0 = u64::from_be_bytes(arr);

    let slice2 = &array[8..16];
    arr.copy_from_slice(slice2);
    tuple.1 = u64::from_be_bytes(arr);

    let slice3 = &array[16..24];
    arr.copy_from_slice(slice3);
    tuple.2 = u64::from_be_bytes(arr);

    let slice4 = &array[24..32];
    arr.copy_from_slice(slice4);
    tuple.3 = u64::from_be_bytes(arr);

    let slice5 = &array[32..40];
    arr.copy_from_slice(slice5);
    tuple.4 = u64::from_be_bytes(arr);

    let slice6 = &array[40..48];
    arr.copy_from_slice(slice6);
    tuple.5 = u64::from_be_bytes(arr);

    let slice7 = &array[48..56];
    arr.copy_from_slice(slice7);
    tuple.6 = u64::from_be_bytes(arr);

    let slice8 = &array[56..64];
    arr.copy_from_slice(slice8);
    tuple.7 = u64::from_be_bytes(arr);

    tuple
}

/// Convert a tuple of u64 values to a 64 bytes array.
fn b512_u64_tuple_to_u8_array(tuple: &(u64, u64, u64, u64, u64, u64, u64, u64)) -> [u8; 64] {
    let mut bytes = [0u8; 64];
    let (t1, t2, t3, t4, t5, t6, t7, t8) = tuple;
    bytes[0..8].copy_from_slice(&t1.to_be_bytes());
    bytes[8..16].copy_from_slice(&t2.to_be_bytes());
    bytes[16..24].copy_from_slice(&t3.to_be_bytes());
    bytes[24..32].copy_from_slice(&t4.to_be_bytes());
    bytes[32..40].copy_from_slice(&t5.to_be_bytes());
    bytes[40..48].copy_from_slice(&t6.to_be_bytes());
    bytes[48..56].copy_from_slice(&t7.to_be_bytes());
    bytes[56..64].copy_from_slice(&t8.to_be_bytes());
    bytes
}

#[cfg(test)]
mod tests_bip32_ed25519 {
    use super::*;

    // Test vectors are converted from CIP-0011
    // https://cips.cardano.org/cip/CIP-0011
    const XPRV1: [u8; 64] = [
        200, 191, 149, 165, 98, 208, 246, 104, 52, 11, 13, 195, 131, 134, 5, 150, 34, 84, 34, 234,
        246, 156, 89, 44, 102, 183, 12, 25, 181, 229, 151, 68, 216, 238, 211, 173, 41, 106, 14, 51,
        53, 217, 219, 231, 210, 32, 13, 82, 86, 83, 210, 195, 255, 75, 225, 13, 74, 150, 225, 78,
        177, 165, 3, 214,
    ];

    const CHAINCODE1: [u8; 32] = [
        98, 56, 179, 184, 207, 42, 180, 226, 223, 22, 246, 228, 154, 15, 134, 223, 246, 201, 237,
        64, 158, 145, 73, 32, 113, 98, 71, 129, 188, 170, 18, 213,
    ];
    const PUBKEY1: &str = "3753d92d88778c4087c3fa59eb748a276eb654164ef23403aeae200ddd554d3e";
    const DATA: &[u8; 4] = b"test";

    #[test]
    fn test_get_public_key() {
        let xprv = XPrv::from_extended_and_chaincode(&XPRV1, &CHAINCODE1);
        let pubk_tuple = get_public_key(&xprv);
        let pubk_hex = format!(
            "{:x}{:x}{:x}{:x}",
            pubk_tuple.0, pubk_tuple.1, pubk_tuple.2, pubk_tuple.3
        );
        assert_eq!(pubk_hex, PUBKEY1);
    }

    #[test]
    fn test_sign_data_and_check_signature() {
        let xprv = XPrv::from_extended_and_chaincode(&XPRV1, &CHAINCODE1);
        let sign_data = sign_data(&xprv, &DATA.to_vec());
        let check_signature = check_signature(&xprv, &DATA.to_vec(), sign_data);
        assert!(check_signature);
    }

    #[test]
    fn test_derive_new_private_key() {
        let xprv = XPrv::from_extended_and_chaincode(&XPRV1, &CHAINCODE1);
        let derived_xprv =
            derive_new_private_key(xprv, "m/1852'/1815'/0'/2/0").expect("Derivation failed");
        assert_eq!(derived_xprv.to_string(), "b8ab42f1aacbcdb3ae858e3a3df88142b3ed27a2d3f432024e0d943fc1e597442d57545d84c8db2820b11509d944093bc605350e60c533b8886a405bd59eed6dcf356648fe9e9219d83e989c8ff5b5b337e2897b6554c1ab4e636de791fe5427");
    }
}
