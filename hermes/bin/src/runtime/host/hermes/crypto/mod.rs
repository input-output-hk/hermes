//! Host - Crypto implementations
//!
#![allow(unused_variables)]

use crate::runtime::extensions::hermes::{
    binary::api::Bstr,
    crypto::api::{
        Ed25519Bip32, Ed25519Bip32PrivateKey, Ed25519Bip32PublicKey, Ed25519Bip32Signature, Host,
        HostEd25519Bip32,
    },
};

/// State
struct State {}

impl HostEd25519Bip32 for State {
    #[doc = " Create a new ED25519-BIP32 Crypto resource"]
    #[doc = " "]
    #[doc = " **Parameters**"]
    #[doc = " "]
    #[doc = " - `private_key` : The key to use, if not supplied one is RANDOMLY generated."]
    fn new(
        &mut self, private_key: Option<Ed25519Bip32PrivateKey>,
    ) -> wasmtime::Result<wasmtime::component::Resource<Ed25519Bip32>> {
        todo!()
    }

    #[doc = " Get the public key for this private key."]
    fn public_key(
        &mut self, self_: wasmtime::component::Resource<Ed25519Bip32>,
    ) -> wasmtime::Result<Ed25519Bip32PublicKey> {
        todo!()
    }

    #[doc = " Sign data with the Private key, and return it."]
    #[doc = " "]
    #[doc = " **Parameters**"]
    #[doc = " "]
    #[doc = " - `data` : The data to sign."]
    fn sign_data(
        &mut self, self_: wasmtime::component::Resource<Ed25519Bip32>, data: Bstr,
    ) -> wasmtime::Result<Ed25519Bip32Signature> {
        todo!()
    }

    #[doc = " Check a signature on a set of data."]
    #[doc = " "]
    #[doc = " **Parameters**"]
    #[doc = " "]
    #[doc = " - `data` : The data to check."]
    #[doc = " - `sig`  : The signature to check."]
    #[doc = " "]
    #[doc = " **Returns**"]
    #[doc = " "]
    #[doc = " - `true` : Signature checked OK."]
    #[doc = " - `false` : Signature check failed."]
    fn check_sig(
        &mut self, self_: wasmtime::component::Resource<Ed25519Bip32>, data: Bstr,
        sig: Ed25519Bip32Signature,
    ) -> wasmtime::Result<bool> {
        todo!()
    }

    #[doc = " Derive a new private key from the current private key."]
    #[doc = " "]
    #[doc = " Note: uses BIP32 HD key derivation."]
    fn derive(
        &mut self, self_: wasmtime::component::Resource<Ed25519Bip32>,
    ) -> wasmtime::Result<wasmtime::component::Resource<Ed25519Bip32>> {
        todo!()
    }

    #[doc = " Create a new RANDOM private key."]
    #[doc = " "]
    #[doc = " Note, this does not need to be used, as the constructor will do this automatically."]
    fn gen_private_key(&mut self) -> wasmtime::Result<Ed25519Bip32PrivateKey> {
        todo!()
    }

    fn drop(&mut self, rep: wasmtime::component::Resource<Ed25519Bip32>) -> wasmtime::Result<()> {
        todo!()
    }
}

impl Host for State {}
