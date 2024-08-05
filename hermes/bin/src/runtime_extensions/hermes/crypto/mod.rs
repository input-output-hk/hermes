//! Crypto runtime extension implementation.

mod bip32_ed25519;
mod bip39;
mod host;
mod state;

/// Advise Runtime Extensions of a new context
pub(crate) fn new_context(_ctx: &crate::runtime_context::HermesRuntimeContext) {}
