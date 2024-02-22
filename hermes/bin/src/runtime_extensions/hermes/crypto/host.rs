//! Crypto host implementation for WASM runtime.

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

impl HostEd25519Bip32 for HermesState {
    /// Create a new ED25519-BIP32 Crypto resource
    ///
    /// **Parameters**
    ///
    /// - `private_key` : The key to use, if not supplied one is RANDOMLY generated.
    fn new(
        &mut self, _private_key: Option<Ed25519Bip32PrivateKey>,
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
        todo!()
    }
}

impl Host for HermesState {}
