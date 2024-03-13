//! Crypto host implementation for WASM runtime.

use bip39::Language;
use wasmtime::component::Resource;

use crate::runtime_extensions::bindings::hermes::crypto::api::Errno;
use crate::wasm::module::ModuleId;
use crate::{
    app::HermesAppName,
    runtime_context::HermesRuntimeContext,
    runtime_extensions::{
        bindings::hermes::{
            binary::api::Bstr,
            crypto::api::{
                Bip32Ed25519, Bip32Ed25519PrivateKey, Bip32Ed25519PublicKey, Bip32Ed25519Signature,
                Host, HostBip32Ed25519, MnemonicPhrase, Passphrase, Path,
            },
        },
        hermes::crypto::{
            bip32_ed25519::get_public_key,
            state::{add_resource, get_resource},
        },
    },
};

use super::{
    bip32_ed25519::{check_signature, derive_new_private_key, sign_data},
    bip39::{generate_new_mnemonic, mnemonic_to_xprv},
};

impl HostBip32Ed25519 for HermesRuntimeContext {
    /// Create a new ED25519-BIP32 Crypto resource
    ///
    /// **Parameters**
    ///
    /// - `mnemonic-phrase` : BIP39 mnemonic, if not supplied one is RANDOMLY generated.
    /// - `passphrase` : Optional BIP39 passphrase.
    fn new(
        &mut self, mnemonic: Option<MnemonicPhrase>, passphrase: Option<Passphrase>,
    ) -> wasmtime::Result<wasmtime::component::Resource<Bip32Ed25519>> {
        // FIXME - Currently not working because of mismatch type
        Ok(generate_resource(
            self.app_name(),
            self.module_id(),
            self.event_name(),
            self.exc_counter(),
            mnemonic,
            passphrase,
        ))
    }

    /// Get the public key for this private key.
    fn public_key(
        &mut self, resource: wasmtime::component::Resource<Bip32Ed25519>,
    ) -> wasmtime::Result<Bip32Ed25519PublicKey> {
       todo!()
    }

    /// Sign data with the Private key, and return it.
    ///
    /// **Parameters**
    ///
    /// - `data` : The data to sign.
    fn sign_data(
        &mut self, resource: wasmtime::component::Resource<Bip32Ed25519>, data: Bstr,
    ) -> wasmtime::Result<Bip32Ed25519Signature> {
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
        &mut self, resource: wasmtime::component::Resource<Bip32Ed25519>, data: Bstr,
        sig: Bip32Ed25519Signature,
    ) -> wasmtime::Result<bool> {
        todo!()
    }

    /// Derive a new private key from the current private key.
    ///
    /// **Parameters**
    ///
    /// - `path` : Derivation path.
    ///
    /// Note: uses BIP32 HD key derivation.
    fn derive(
        &mut self, resource: wasmtime::component::Resource<Bip32Ed25519>, path: Path,
    ) -> wasmtime::Result<wasmtime::component::Resource<Bip32Ed25519>> {
        todo!()
    }

    /// Create a new RANDOM private key.
    ///
    /// Note, this does not need to be used, as the constructor will do this
    /// automatically.
    fn gen_private_key(&mut self) -> wasmtime::Result<Bip32Ed25519PrivateKey> {
        todo!()
    }

    fn drop(&mut self, res: wasmtime::component::Resource<Bip32Ed25519>) -> wasmtime::Result<()> {
        // self.hermes.crypto.private_key.drop(rep.rep()).unwrap_or(());

        Ok(())
    }
}

fn generate_resource(
    app_name: &HermesAppName, module_id: &ModuleId, event_name: &str, counter: u32,
    mnemonic: Option<MnemonicPhrase>, passphrase: Option<Passphrase>,
) -> Result<wasmtime::component::Resource<Bip32Ed25519>, Errno> {
    let xprv = match mnemonic {
        // If mnemonic is supplied, use it to generate xprv.
        Some(mnemonic) => {
            let passphrase = passphrase.unwrap_or_default();
            mnemonic_to_xprv(&mnemonic.join(" "), &passphrase.join(" "))
        },
        None => {
            // If mnemonic is not supplied, generate a new one
            // then generate xprv.
            let mnemonic = generate_new_mnemonic(12, Vec::new(), Language::English)
                .expect("Failed to generate mnemonic"); // FIXME - Should be a proper error
            mnemonic_to_xprv(&mnemonic, "")
        },
    };

    let resource = match xprv {
        // If xprv is generated, add it to the state and return the resource.
        Ok(xprv) => {
            match add_resource(app_name, module_id, event_name, &counter, xprv) {
                Some(id) => Ok(Resource::new_own(id)),
                None => todo!() // FIXME - Should be a proper error
            }
        },
        Err(e) => Err(e), 
    };

    resource
}

impl Host for HermesRuntimeContext {}
