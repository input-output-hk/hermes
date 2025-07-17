//! Hermes runtime extensions implementations - HERMES custom extensions

use crate::runtime_context::HermesRuntimeContext;

pub(crate) mod binary;
pub(crate) mod cardano;
pub(crate) mod cbor;
pub(crate) mod cron;
pub(crate) mod crypto;
pub(crate) mod hash;
pub(crate) mod http_gateway;
pub(crate) mod http_request;
pub(crate) mod init;
pub mod integration_test;
pub(crate) mod ipfs;
pub(crate) mod json;
pub(crate) mod kv_store;
pub(crate) mod localtime;
pub(crate) mod logging;
pub(crate) mod sqlite;

/// Advise Runtime Extensions of a new context
pub(crate) fn new_context(ctx: &HermesRuntimeContext) {
    binary::new_context(ctx);
    cardano::new_context(ctx);
    cbor::new_context(ctx);
    cron::new_context(ctx);
    crypto::new_context(ctx);
    hash::new_context(ctx);
    init::new_context(ctx);
    ipfs::new_context(ctx);
    json::new_context(ctx);
    kv_store::new_context(ctx);
    localtime::new_context(ctx);
    logging::new_context(ctx);
    sqlite::new_context(ctx);
    http_gateway::new_context(ctx);
    http_request::new_context(ctx);
}
