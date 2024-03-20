//! Crypto runtime extension implementation.

use self::state::{get_state, set_state};

mod bip32_ed25519;
mod bip39;
mod host;
mod state;

/// Advise Runtime Extensions of a new context
pub(crate) fn new_context(ctx: &crate::runtime_context::HermesRuntimeContext) {
    // check whether it exist
    let state = get_state();
    if state.contains_key(ctx.app_name()) {
        set_state(ctx.app_name().clone());
    }
}
