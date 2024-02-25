//! Hermes runtime extensions implementations - HERMES custom extensions

use crate::runtime_extensions::state::Stateful;

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

/// Hermes extensions state
pub(crate) struct State {
    /// Binary extensions state
    pub(crate) _binary: binary::State,
    /// Cardano extensions state
    pub(crate) _cardano: cardano::State,
    /// CBOR extensions state
    pub(crate) _cbor: cbor::State,
    /// Cron extensions state
    pub(crate) _cron: cron::State,
    /// Crypto extensions state
    pub(crate) _crypto: crypto::State,
    /// Hash extensions state
    pub(crate) _hash: hash::State,
    /// Init extensions state
    pub(crate) init: init::State,
    /// JSON extensions state
    pub(crate) _json: json::State,
    /// KV store extensions state
    pub(crate) _kv_store: kv_store::State,
    /// Localtime extensions state
    pub(crate) _localtime: localtime::State,
    /// Logging extensions state
    pub(crate) _logging: logging::State,
}

impl Stateful for State {
    fn new() -> Self {
        Self {
            _binary: binary::State::new(),
            _cardano: cardano::State::new(),
            _cbor: cbor::State::new(),
            _cron: cron::State::new(),
            _crypto: crypto::State::new(),
            _hash: hash::State::new(),
            init: init::State::new(),
            _json: json::State::new(),
            _kv_store: kv_store::State::new(),
            _localtime: localtime::State::new(),
            _logging: logging::State::new(),
        }
    }
}
