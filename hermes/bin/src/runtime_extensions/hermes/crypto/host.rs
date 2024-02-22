//! Crypto host implementation for WASM runtime.

use std::u8;

// cspell: words prvk pubk
use ed25519_bip32::{DerivationScheme, Signature, XPrv};

use crate::{
    runtime_extensions::bindings::hermes::{
        binary::api::Bstr,
        crypto::api::{
            Ed25519Bip32, Ed25519Bip32PrivateKey, Ed25519Bip32PublicKey, Ed25519Bip32Signature,
            Host, HostEd25519Bip32,
        },
    },
    state::HermesState,
};

use super::Storage;

fn check_context(ctx: &HermesState, storage: Storage) -> bool {
    let context = ctx.ctx;
    let context_app_name = context.app_name();
    let context_module_id = context.module_id();
    let context_event_name = context.event_name();

    // - If app name is found in storage
    if let Some(storage_app) = storage.get(context_app_name) {
        // If module id is found in storage
        if let Some(storage_module_id) = storage_app.get(context_module_id) {
            // If event name is found in storage
            if let Some(storage_event_name) = storage_module_id.get(&context_event_name) {
                if storage_event_name.current_exec_count == context.counter() {
                    return true;
                } else {
                    storage_event_name.current_exec_count = context.counter();
                    todo!()
                }
            }
        }
    }
    return false;
}

// The tuple should contain only u64 values
fn b256_u64_tuple_to_u8_array(tuple: &(u64, u64, u64, u64)) -> [u8; 32] {
    let mut bytes = [0u8; 32];
    let (a, b, c, d) = tuple;
    bytes[0..8].copy_from_slice(&a.to_le_bytes());
    bytes[8..16].copy_from_slice(&b.to_le_bytes());
    bytes[16..24].copy_from_slice(&c.to_le_bytes());
    bytes[24..32].copy_from_slice(&d.to_le_bytes());
    bytes
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

fn u8_array_to_u64_tuple(bytes: &[u8; 32]) -> (u64, u64, u64, u64) {
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
fn b512_u8_array_to_u64_tuple(bytes: &[u8; 64]) -> (u64, u64, u64, u64, u64, u64, u64, u64) {
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
const CHAIN_CODE: [u8; 32] = [0u8; 32];

impl HostEd25519Bip32 for HermesState {
    /// Create a new ED25519-BIP32 Crypto resource
    ///
    /// **Parameters**
    ///
    /// - `private_key` : The key to use, if not supplied one is RANDOMLY generated.
    fn new(
        &mut self, private_key: Option<Ed25519Bip32PrivateKey>,
    ) -> wasmtime::Result<wasmtime::component::Resource<Ed25519Bip32>> {
        match private_key {
            // Private key is supplied
            Some(private_key) => {
                let pk = b256_u64_tuple_to_u8_array(&private_key);
                let xprv = XPrv::from_nonextended_force(&pk, &CHAIN_CODE);
                todo!()
                // Resource::new_own(self.hermes.crypto.storage.insert(key, value))
                //     Ok(_) => Ok(Resource::new_own(
                //         self.hermes.crypto.private_key.add(),
                //     )),
                //     Err(e) => Err(wasmtime::Error::new(e)),
                // }
            },
            // TODO - Generate new private key
            None => todo!(),
        }
    }

    /// Get the public key for this private key.
    fn public_key(
        &mut self, resource: wasmtime::component::Resource<Ed25519Bip32>,
    ) -> wasmtime::Result<Ed25519Bip32PublicKey> {
        let res = check_context(self, self.hermes.crypto.storage);
        match self
            .hermes
            .crypto
            .extended_key_map
            .index_to_priv
            .get(&resource.rep())
        {
            // The given private key exists
            Some(private_key) => match XPrv::from_bytes_verified(private_key.clone()) {
                Ok(prvk) => {
                    let pubk = XPrv::public(&prvk);
                    Ok(u8_array_to_u64_tuple(pubk.public_key_bytes()))
                },
                Err(e) => Err(wasmtime::Error::new(e)),
            },
            // TODO - create custom error, private key not found
            None => todo!(),
        }
    }

    /// Sign data with the Private key, and return it.
    ///
    /// **Parameters**
    ///
    /// - `data` : The data to sign.
    fn sign_data(
        &mut self, resource: wasmtime::component::Resource<Ed25519Bip32>, data: Bstr,
    ) -> wasmtime::Result<Ed25519Bip32Signature> {
        let res = check_context(self, self.hermes.crypto.storage);
        match self
            .hermes
            .crypto
            .extended_key_map
            .index_to_priv
            .get(&resource.rep())
        {
            // The given private key exists
            Some(private_key) => match XPrv::from_bytes_verified(private_key.clone()) {
                Ok(prvk) => {
                    let sig: Signature<&Bstr> = prvk.sign(&data);
                    Ok(b512_u8_array_to_u64_tuple(sig.to_bytes()))
                },
                Err(e) => Err(wasmtime::Error::new(e)),
            },
            // TODO - create custom error, private key not found
            None => todo!(),
        }
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
        &mut self, resource: wasmtime::component::Resource<Ed25519Bip32>, data: Bstr,
        sig: Ed25519Bip32Signature,
    ) -> wasmtime::Result<bool> {
        let res = check_context(self, self.hermes.crypto.storage);
        match self
            .hermes
            .crypto
            .extended_key_map
            .index_to_priv
            .get(&resource.rep())
        {
            // The given private key exists
            Some(private_key) => {
                let signature: Signature<[u8; 64]> =
                    Signature::from_bytes(b512_u64_tuple_to_u8_array(&sig));
                match XPrv::from_bytes_verified(private_key.clone()) {
                    Ok(prvk) => Ok(prvk.verify(&data, &signature)),
                    Err(e) => Err(wasmtime::Error::new(e)),
                }
            },
            // TODO - create custom error, private key not found
            None => todo!(),
        }
    }

    /// Derive a new private key from the current private key.
    ///
    /// Note: uses BIP32 HD key derivation.
    fn derive(
        &mut self, resource: wasmtime::component::Resource<Ed25519Bip32>,
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

    fn drop(&mut self, rep: wasmtime::component::Resource<Ed25519Bip32>) -> wasmtime::Result<()> {
        // self.hermes.crypto.private_key.drop(rep.rep()).unwrap_or(());
        Ok(())
    }
}

impl Host for HermesState {}
