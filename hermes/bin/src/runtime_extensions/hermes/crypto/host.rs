//! Crypto host implementation for WASM runtime.

use super::{
    bip32_ed25519::{check_signature, derive_new_private_key, get_public_key, sign_data},
    bip39::{generate_new_mnemonic, mnemonic_to_xprv},
    state::get_state,
};
use crate::{
    runtime_context::HermesRuntimeContext,
    runtime_extensions::bindings::hermes::{
        binary::api::Bstr,
        crypto::api::{
            Bip32Ed25519, Bip32Ed25519PublicKey, Bip32Ed25519Signature, Errno, Host,
            HostBip32Ed25519, MnemonicPhrase, Passphrase, Path,
        },
    },
};

impl HostBip32Ed25519 for HermesRuntimeContext {
    /// Create a new ED25519-BIP32 Crypto resource
    ///
    /// **Parameters**
    ///
    /// - `mnemonic-phrase` : BIP39 mnemonic.
    /// - `passphrase` : Optional BIP39 passphrase.
    fn new(
        &mut self, mnemonic: MnemonicPhrase, passphrase: Option<Passphrase>,
    ) -> wasmtime::Result<wasmtime::component::Resource<Bip32Ed25519>> {
        let passphrase = passphrase.unwrap_or_default();
        // TODO(bkioshn): https://github.com/input-output-hk/hermes/issues/183
        let xprv = mnemonic_to_xprv(&mnemonic.join(" "), &passphrase.join(" "))
            .map_err(|e| wasmtime::Error::msg(e.to_string()))?;

        let app_state = get_state().get_app_state(self.app_name())?;
        Ok(app_state.create_resource(xprv))
    }

    /// Get the public key for this private key.
    fn public_key(
        &mut self, resource: wasmtime::component::Resource<Bip32Ed25519>,
    ) -> wasmtime::Result<Bip32Ed25519PublicKey> {
        let mut app_state = get_state().get_app_state(self.app_name())?;
        let private_key = app_state.get_object(&resource)?;
        let public_key = get_public_key(&private_key);
        Ok(public_key)
    }

    /// Sign data with the Private key, and return it.
    ///
    /// **Parameters**
    ///
    /// - `data` : The data to sign.
    fn sign_data(
        &mut self, resource: wasmtime::component::Resource<Bip32Ed25519>, data: Bstr,
    ) -> wasmtime::Result<Bip32Ed25519Signature> {
        let mut app_state = get_state().get_app_state(self.app_name())?;
        let private_key = app_state.get_object(&resource)?;
        let sig = sign_data(&private_key, &data);
        Ok(sig)
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
        let mut app_state = get_state().get_app_state(self.app_name())?;
        let private_key = app_state.get_object(&resource)?;
        let check_sig = check_signature(&private_key, &data, sig);
        Ok(check_sig)
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
        let mut app_state = get_state().get_app_state(self.app_name())?;

        let private_key = app_state.get_object(&resource)?;
        // TODO(bkioshn): https://github.com/input-output-hk/hermes/issues/183
        let new_private_key = derive_new_private_key(private_key.clone(), &path)
            .map_err(|_| wasmtime::Error::msg("Error deriving new private key"))?;
        drop(private_key);
        Ok(app_state.create_resource(new_private_key))
    }

    fn drop(&mut self, res: wasmtime::component::Resource<Bip32Ed25519>) -> wasmtime::Result<()> {
        let app_state = get_state().get_app_state(self.app_name())?;
        app_state.delete_resource(res)?;
        Ok(())
    }
}

impl Host for HermesRuntimeContext {
    /// # Generate BIP39 Mnemonic Function
    ///
    /// Generate a new BIP39 mnemonic phrase with the given
    /// size, prefix and language.
    ///
    /// ## Parameters
    ///
    /// `size` : The size of the mnemonic. Must be a multiple of 3 and in the range of 12
    /// - 24.
    /// `prefix` : The prefix for the mnemonic. Must be a list of 1 - 3 words.
    /// `language` : Optional. The language to use for the mnemonic.
    ///              If not provided, the default language is used.
    ///
    /// ## Returns
    ///
    /// - Either a list of mnemonic words.
    /// - Or an error if the mnemonic could not be generated:
    ///     - `prefix-too-long` : The prefix is longer than the maximum allowed length,
    ///       max is 3.
    ///     - `invalid-mnemonic-length` : The mnemonic length is not a multiple of 3 or
    ///       not in the range of 12 - 24.
    ///     - `word-not-found` : A word in the mnemonic is not found in the word list.
    ///     - `generate-entropy-failed` : Failed to generate entropy.
    fn generate_mnemonic(
        &mut self, size: u8, prefix: Vec<String>, language: Option<String>,
    ) -> wasmtime::Result<Result<Vec<String>, Errno>> {
        Ok(generate_new_mnemonic(size.into(), prefix, language))
    }
}
