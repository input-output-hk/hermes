//! Crypto host implementation for WASM runtime.

use ed25519_bip32::{DerivationScheme, Signature, XPrv};
use wasmtime::component::Resource;

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

use super::Ed25519Bip32Struct;

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
            Some(private_key) => {
                let check = XPrv::from_slice_verified(&private_key);
                let pk = Ed25519Bip32Struct { private_key };
                if check.is_ok() {
                    return Ok(Resource::new_own(self.hermes.crypto.private_key.new(pk)));
                } else {
                    todo!()
                }
            },
            None => todo!(),
        }
    }

    /// Get the public key for this private key.
    fn public_key(
        &mut self, resource: wasmtime::component::Resource<Ed25519Bip32>,
    ) -> wasmtime::Result<Ed25519Bip32PublicKey> {
        let private_key = self
            .hermes
            .crypto
            .private_key
            .get(resource.rep())
            .unwrap()
            .private_key
            .clone();
        let check = XPrv::from_slice_verified(&private_key);
        if check.is_ok() {
            let pubk = XPrv::public(&check.unwrap());
            Ok(pubk.public_key_slice().to_vec())
        } else {
            todo!()
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
        let private_key = self
        .hermes
        .crypto
        .private_key
        .get(resource.rep())
        .unwrap()
        .private_key
        .clone();
        let check = XPrv::from_slice_verified(&private_key);
        if check.is_ok() {
            let sig: Signature<&Bstr> = check.unwrap().sign(&data);
            return Ok(sig.to_bytes().to_vec());
        }
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
        &mut self, resource: wasmtime::component::Resource<Ed25519Bip32>, data: Bstr,
        sig: Ed25519Bip32Signature,
    ) -> wasmtime::Result<bool> {
        let private_key = self
        .hermes
        .crypto
        .private_key
        .get(resource.rep())
        .unwrap()
        .private_key
        .clone();
        let check = XPrv::from_slice_verified(&private_key);
        let signature: Signature<Bstr>  = Signature::from_slice(&sig).unwrap();
        if check.is_ok() {
            return Ok(check.unwrap().verify(&data, &signature));
        }
        todo!()
    }

    /// Derive a new private key from the current private key.
    ///
    /// Note: uses BIP32 HD key derivation.
    fn derive(
        &mut self, resource: wasmtime::component::Resource<Ed25519Bip32>,
    ) -> wasmtime::Result<wasmtime::component::Resource<Ed25519Bip32>> {
        let private_key = self
        .hermes
        .crypto
        .private_key
        .get(resource.rep())
        .unwrap()
        .private_key
        .clone();
        let check = XPrv::from_slice_verified(&private_key);
        if check.is_ok() {
            // Recheck the index
            let new_key = check.unwrap().derive(DerivationScheme::V2, 0);
            let r_new_key = self.new(Some(new_key.as_ref().to_vec()));
            return r_new_key;
        }
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
        Ok(self.hermes.crypto.private_key.drop(rep.rep()).unwrap_or(()))
    }
}

impl Host for HermesState {}
