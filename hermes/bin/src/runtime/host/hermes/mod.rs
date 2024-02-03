//! Runtime modules - extensions - HERMES custom extensions

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

use crate::runtime::extensions::NewState;

#[allow(dead_code)]
/// Hermes extensions state
pub(crate) struct State {
    /// Binary extensions state
    binary: binary::State,
    /// Cardano extensions state
    cardano: cardano::State,
    /// CBOR extensions state
    cbor: cbor::State,
    /// Cron extensions state
    cron: cron::State,
    /// Crypto extensions state
    crypto: crypto::State,
    /// Hash extensions state
    hash: hash::State,
    /// Init extensions state
    init: init::State,
    /// JSON extensions state
    json: json::State,
    /// KV store extensions state
    kv_store: kv_store::State,
    /// Localtime extensions state
    localtime: localtime::State,
    /// Logging extensions state
    logging: logging::State,
}

impl NewState for State {
    fn new(ctx: &crate::wasm::context::Context) -> Self {
        Self {
            binary: binary::State::new(ctx),
            cardano: cardano::State::new(ctx),
            cbor: cbor::State::new(ctx),
            cron: cron::State::new(ctx),
            crypto: crypto::State::new(ctx),
            hash: hash::State::new(ctx),
            init: init::State::new(ctx),
            json: json::State::new(ctx),
            kv_store: kv_store::State::new(ctx),
            localtime: localtime::State::new(ctx),
            logging: logging::State::new(ctx),
        }
    }
}
