//! Hermes runtime extensions implementations - HERMES custom extensions

use crate::runtime_context::HermesRuntimeContext;

pub(crate) mod binary;
pub(crate) mod cardano;
pub(crate) mod cbor;
pub(crate) mod cron;
pub(crate) mod crypto;
pub(crate) mod hash;
pub(crate) mod init;
pub(crate) mod json;
pub(crate) mod kv_store;
pub(crate) mod localtime;
pub(crate) mod logging;

/// Advise Runtime Extensions of a new context
pub(crate) fn new_context(ctx: &HermesRuntimeContext) {
    binary::new_context(ctx);
    cardano::new_context(ctx);
    cbor::new_context(ctx);
    cron::new_context(ctx);
    crypto::new_context(ctx);
    hash::new_context(ctx);
    init::new_context(ctx);
    json::new_context(ctx);
    kv_store::new_context(ctx);
    localtime::new_context(ctx);
    logging::new_context(ctx);
}
