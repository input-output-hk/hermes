//! Crypto state

use ed25519_bip32::XPrv;
use once_cell::sync::Lazy;

use crate::{
    app::ApplicationName,
    runtime_extensions::{
        bindings::hermes::crypto::api::Bip32Ed25519, resource_manager::ApplicationResourceManager,
    },
};

/// Map of app name to resource holder
type State = ApplicationResourceManager<Bip32Ed25519, XPrv>;

/// Global state to hold the resources.
static CRYPTO_INTERNAL_STATE: Lazy<State> = Lazy::new(ApplicationResourceManager::new);

/// Get the resource from the state using id if possible.
pub(crate) fn get_resource(
    app_name: &ApplicationName, resource: &wasmtime::component::Resource<Bip32Ed25519>,
) -> wasmtime::Result<XPrv> {
    CRYPTO_INTERNAL_STATE.get_object(app_name.clone(), resource)
}

/// Add the resource of `XPrv` to the state if possible.
/// Return the id if successful.
pub(crate) fn add_resource(
    app_name: &ApplicationName, xprv: XPrv,
) -> wasmtime::component::Resource<Bip32Ed25519> {
    CRYPTO_INTERNAL_STATE.create_resource(app_name.clone(), xprv)
}

/// Delete the resource from the state using id if possible.
pub(crate) fn delete_resource(
    app_name: &ApplicationName, resource: wasmtime::component::Resource<Bip32Ed25519>,
) {
    CRYPTO_INTERNAL_STATE.delete_resource(app_name.clone(), resource);
}
