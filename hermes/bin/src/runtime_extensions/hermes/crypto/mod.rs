//! Crypto runtime extension implementation.

use self::state::set_state;

mod host;
mod bip32_ed25519;
mod bip39;
mod state;

/// Advise Runtime Extensions of a new context
pub(crate) fn new_context(ctx: &crate::runtime_context::HermesRuntimeContext) {
    set_state(ctx.app_name().clone(), ctx.module_id().clone(),ctx.event_name(), ctx.exc_counter());
}
