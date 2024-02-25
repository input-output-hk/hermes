//! Hermes runtime extensions implementations - WASI standard extensions

use crate::runtime_extensions::state::Stateful;

pub(crate) mod cli;
pub(crate) mod clocks;
pub(crate) mod filesystem;
pub(crate) mod http;
pub(crate) mod io;
pub(crate) mod random;

/// WASI State
pub(crate) struct State {
    /// WASI CLI State
    _cli: cli::State,
    /// WASI Clock State
    _clocks: clocks::State,
    /// WASI Filesystem State
    _filesystem: filesystem::State,
    /// WASI HTTP State
    _http: http::State,
    /// WASI IO State
    _io: io::State,
    /// WASI Random State
    _random: random::State,
}

impl Stateful for State {
    fn new() -> Self {
        Self {
            _cli: cli::State::new(),
            _clocks: clocks::State::new(),
            _filesystem: filesystem::State::new(),
            _http: http::State::new(),
            _io: io::State::new(),
            _random: random::State::new(),
        }
    }
}
