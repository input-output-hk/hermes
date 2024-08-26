//! Crypto state

use ed25519_bip32::XPrv;
use once_cell::sync::Lazy;

use crate::runtime_extensions::{
    bindings::hermes::crypto::api::Bip32Ed25519, resource_manager::ApplicationResourceStorage,
};

/// Map of app name to resource holder
pub(super) type State = ApplicationResourceStorage<Bip32Ed25519, XPrv>;

/// Global state to hold the resources.
static CRYPTO_STATE: Lazy<State> = Lazy::new(ApplicationResourceStorage::new);

/// Get the crypto state.
pub(super) fn get_state() -> &'static State {
    &CRYPTO_STATE
}
