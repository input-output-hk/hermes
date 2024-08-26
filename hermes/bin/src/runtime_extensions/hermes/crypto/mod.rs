//! Crypto runtime extension implementation.

mod bip32_ed25519;
mod bip39;
mod host;
mod state;

/// Advise Runtime Extensions of a new context
pub(crate) fn new_context(ctx: &crate::runtime_context::HermesRuntimeContext) {
    state::get_state().add_app(ctx.app_name().clone());
}
