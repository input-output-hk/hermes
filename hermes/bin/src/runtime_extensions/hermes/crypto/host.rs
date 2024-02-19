//! Crypto host implementation for WASM runtime.

// cspell: words prvk pubk
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
                match XPrv::from_slice_verified(&private_key) {
                    Ok(_) => {
                        Ok(Resource::new_own(
                            self.hermes.crypto.private_key.add(private_key),
                        ))
                    },
                    Err(e) => Err(wasmtime::Error::new(e)),
                }
            },
            // TODO - Generate new private key
            None => todo!(),
        }
    }

    /// Get the public key for this private key.
    fn public_key(
        &mut self, resource: wasmtime::component::Resource<Ed25519Bip32>,
    ) -> wasmtime::Result<Ed25519Bip32PublicKey> {
        match self.hermes.crypto.private_key.get(resource.rep()) {
            // The given private key exists
            Some(private_key) => {
                let prvk = XPrv::from_slice_verified(private_key);
                match prvk {
                    Ok(prvk) => {
                        let pubk = XPrv::public(&prvk);
                        Ok(pubk.public_key_slice().to_vec())
                    },
                    Err(e) => Err(wasmtime::Error::new(e)),
                }
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
        match self.hermes.crypto.private_key.get(resource.rep()) {
            // The given private key exists
            Some(private_key) => {
                let prvk = XPrv::from_slice_verified(private_key);
                match prvk {
                    Ok(prvk) => {
                        let sig: Signature<&Bstr> = prvk.sign(&data);
                        return Ok(sig.to_bytes().into());
                    },
                    Err(e) => Err(wasmtime::Error::new(e)),
                }
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
        match self.hermes.crypto.private_key.get(resource.rep()) {
            // The given private key exists
            Some(private_key) => {
                let prvk = XPrv::from_slice_verified(private_key);
                let signature: Signature<Bstr> = match Signature::from_slice(&sig) {
                    Ok(sig) => sig,
                    Err(e) => return Err(wasmtime::Error::new(e)),
                };
                match prvk {
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
        match self.hermes.crypto.private_key.get(resource.rep()) {
            // The given private key exists
            Some(private_key) => {
                let prvk = XPrv::from_slice_verified(private_key);
                match prvk {
                    // TODO - Recheck the index
                    Ok(prvk) => {
                        let new_derive_key = prvk.derive(DerivationScheme::V2, 0);
                        return self.new(Some(new_derive_key.as_ref().to_vec()));
                    },
                    Err(e) => Err(wasmtime::Error::new(e)),
                }
            },
            // TODO - create custom error, private key not found
            None => todo!(),
        }
    }

    /// Create a new RANDOM private key.
    ///
    /// Note, this does not need to be used, as the constructor will do this
    /// automatically.
    fn gen_private_key(&mut self) -> wasmtime::Result<Ed25519Bip32PrivateKey> {
        todo!()
    }

    fn drop(&mut self, rep: wasmtime::component::Resource<Ed25519Bip32>) -> wasmtime::Result<()> {
        self.hermes.crypto.private_key.drop(rep.rep()).unwrap_or(());
        Ok(())
    }
}

impl Host for HermesState {}
