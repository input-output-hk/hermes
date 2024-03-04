//! Crypto host implementation for WASM runtime.

use std::u8;


// cspell: words prvk pubk
use crate::{
    runtime_extensions::bindings::hermes::{
        binary::api::Bstr,
        crypto::api::{
            Ed25519Bip32, Ed25519Bip32PrivateKey, Ed25519Bip32PublicKey, Ed25519Bip32Signature,
            Host, HostEd25519Bip32, Passphrase, MnemonicPhrase
        },
    },
    state::HermesState,
};
// use ed25519_bip32::{DerivationIndex, DerivationScheme, Signature, XPrv};

// The tuple should contain only u64 values
fn _b256_u64_tuple_to_u8_array(tuple: &(u64, u64, u64, u64)) -> [u8; 32] {
    let mut bytes = [0u8; 32];
    let (a, b, c, d) = tuple;
    bytes[0..8].copy_from_slice(&a.to_le_bytes());
    bytes[8..16].copy_from_slice(&b.to_le_bytes());
    bytes[16..24].copy_from_slice(&c.to_le_bytes());
    bytes[24..32].copy_from_slice(&d.to_le_bytes());
    bytes
}

fn _b512_u64_tuple_to_u8_array(tuple: &(u64, u64, u64, u64, u64, u64, u64, u64)) -> [u8; 64] {
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

fn _u8_array_to_u64_tuple(bytes: &[u8; 32]) -> (u64, u64, u64, u64) {
    let mut a_bytes = [0u8; 8];
    let mut b_bytes = [0u8; 8];
    let mut c_bytes = [0u8; 8];
    let mut d_bytes = [0u8; 8];

    a_bytes.copy_from_slice(&bytes[0..8]);
    b_bytes.copy_from_slice(&bytes[8..16]);
    c_bytes.copy_from_slice(&bytes[16..24]);
    d_bytes.copy_from_slice(&bytes[24..32]);

    let a = u64::from_le_bytes(a_bytes);
    let b = u64::from_le_bytes(b_bytes);
    let c = u64::from_le_bytes(c_bytes);
    let d = u64::from_le_bytes(d_bytes);

    (a, b, c, d)
}
fn _b512_u8_array_to_u64_tuple(bytes: &[u8; 64]) -> (u64, u64, u64, u64, u64, u64, u64, u64) {
    let mut a_bytes = [0u8; 8];
    let mut b_bytes = [0u8; 8];
    let mut c_bytes = [0u8; 8];
    let mut d_bytes = [0u8; 8];
    let mut e_bytes = [0u8; 8];
    let mut f_bytes = [0u8; 8];
    let mut g_bytes = [0u8; 8];
    let mut h_bytes = [0u8; 8];

    a_bytes.copy_from_slice(&bytes[0..8]);
    b_bytes.copy_from_slice(&bytes[8..16]);
    c_bytes.copy_from_slice(&bytes[16..24]);
    d_bytes.copy_from_slice(&bytes[24..32]);
    e_bytes.copy_from_slice(&bytes[32..40]);
    f_bytes.copy_from_slice(&bytes[40..48]);
    g_bytes.copy_from_slice(&bytes[48..56]);
    h_bytes.copy_from_slice(&bytes[56..64]);

    let a = u64::from_le_bytes(a_bytes);
    let b = u64::from_le_bytes(b_bytes);
    let c = u64::from_le_bytes(c_bytes);
    let d = u64::from_le_bytes(d_bytes);
    let e = u64::from_le_bytes(e_bytes);
    let f = u64::from_le_bytes(f_bytes);
    let g = u64::from_le_bytes(g_bytes);
    let h = u64::from_le_bytes(h_bytes);

    (a, b, c, d, e, f, g, h)
}

// Remove this chain code
const _CHAIN_CODE: [u8; 32] = [0u8; 32];
impl HostEd25519Bip32 for HermesState {
    /// Create a new ED25519-BIP32 Crypto resource
    ///
    /// **Parameters**
    ///
    /// - `private_key` : The key to use, if not supplied one is RANDOMLY generated.
    fn new(
        &mut self, _mnemonic: Option<MnemonicPhrase>, _passphrase: Option<Passphrase>,
    ) -> wasmtime::Result<wasmtime::component::Resource<Ed25519Bip32>> {
        todo!()
    }

    /// Get the public key for this private key.
    fn public_key(
        &mut self, _resource: wasmtime::component::Resource<Ed25519Bip32>,
    ) -> wasmtime::Result<Ed25519Bip32PublicKey> {
        todo!()
    }

    /// Sign data with the Private key, and return it.
    ///
    /// **Parameters**
    ///
    /// - `data` : The data to sign.
    fn sign_data(
        &mut self, _resource: wasmtime::component::Resource<Ed25519Bip32>, _data: Bstr,
    ) -> wasmtime::Result<Ed25519Bip32Signature> {
        todo!()
    }

    /// Check a signature on a set of data.
    ///
    /// **Parameters**
    ///
    /// - `data` : The data to check.
    /// - `sig`  : The signature to check.
    ///
    /// **Returns**
    ///
    /// - `true` : Signature checked OK.
    /// - `false` : Signature check failed.
    fn check_sig(
        &mut self, _resource: wasmtime::component::Resource<Ed25519Bip32>, _data: Bstr,
        _sig: Ed25519Bip32Signature,
    ) -> wasmtime::Result<bool> {
        todo!()
    }

    /// Derive a new private key from the current private key.
    ///
    /// Note: uses BIP32 HD key derivation.
    fn derive(
        &mut self, _resource: wasmtime::component::Resource<Ed25519Bip32>,
    ) -> wasmtime::Result<wasmtime::component::Resource<Ed25519Bip32>> {
        // match self.hermes.crypto.private_key.get(resource.rep()) {
        //     // The given private key exists
        //     Some(private_key) => {
        //         let prvk = XPrv::from_slice_verified(private_key);
        //         match prvk {
        //             // TODO - Recheck the index
        //             Ok(prvk) => {
        //                 let new_derive_key = prvk.derive(DerivationScheme::V2, 0);
        //                 return self.new(Some(new_derive_key.as_ref().to_vec()));
        //             },
        //             Err(e) => Err(wasmtime::Error::new(e)),
        //         }
        //     },
        //     // TODO - create custom error, private key not found
        //     None => todo!(),
        // }
        todo!()
    }

    /// Create a new RANDOM private key.
    ///
    /// Note, this does not need to be used, as the constructor will do this
    /// automatically.
    fn gen_private_key(&mut self) -> wasmtime::Result<Ed25519Bip32PrivateKey> {
        todo!()
    }

    fn drop(&mut self, _rep: wasmtime::component::Resource<Ed25519Bip32>) -> wasmtime::Result<()> {
        // self.hermes.crypto.private_key.drop(rep.rep()).unwrap_or(());
        Ok(())
    }
}

impl Host for HermesState {}
