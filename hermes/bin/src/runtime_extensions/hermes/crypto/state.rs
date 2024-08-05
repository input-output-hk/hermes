//! Crypto state

use ed25519_bip32::XPrv;
use once_cell::sync::Lazy;

use crate::runtime_extensions::{
    bindings::hermes::crypto::api::Bip32Ed25519, resource_manager::ApplicationResourceManager,
};

/// Map of app name to resource holder
type State = ApplicationResourceManager<Bip32Ed25519, XPrv>;

/// Global state to hold the resources.
static CRYPTO_STATE: Lazy<State> = Lazy::new(ApplicationResourceManager::new);

/// Get the crypto state.
pub(crate) fn get_state() -> &'static State {
    &CRYPTO_STATE
}
