//! Hermes runtime extensions implementations - HERMES custom extensions

use crate::runtime_extensions::state::{Context, Stateful};

pub mod binary;
pub mod cardano;
pub mod cbor;
pub mod cron;
pub mod crypto;
pub mod hash;
pub mod init;
pub mod integration_test;
pub mod json;
pub mod kv_store;
pub mod localtime;
pub mod logging;

/// Hermes extensions state
pub struct State {
    /// Binary extensions state
    _binary: binary::State,
    /// Cardano extensions state
    _cardano: cardano::State,
    /// CBOR extensions state
    _cbor: cbor::State,
    /// Cron extensions state
    cron: cron::State,
    /// Crypto extensions state
    _crypto: crypto::State,
    /// Hash extensions state
    _hash: hash::State,
    /// Init extensions state
    _init: init::State,
    /// JSON extensions state
    _json: json::State,
    /// KV store extensions state
    _kv_store: kv_store::State,
    /// Localtime extensions state
    _localtime: localtime::State,
    /// Logging extensions state
    _logging: logging::State,
}

impl Stateful for State {
    fn new(ctx: &Context) -> Self {
        Self {
            _binary: binary::State::new(ctx),
            _cardano: cardano::State::new(ctx),
            _cbor: cbor::State::new(ctx),
            cron: cron::State::new(ctx),
            _crypto: crypto::State::new(ctx),
            _hash: hash::State::new(ctx),
            _init: init::State::new(ctx),
            _json: json::State::new(ctx),
            _kv_store: kv_store::State::new(ctx),
            _localtime: localtime::State::new(ctx),
            _logging: logging::State::new(ctx),
        }
    }
}
