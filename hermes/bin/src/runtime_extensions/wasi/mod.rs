//! Hermes runtime extensions implementations - WASI standard extensions

use crate::{event_queue::HermesEventQueueIn, runtime_extensions::state::Stateful};

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
    fn new(event_queue_in: &HermesEventQueueIn) -> Self {
        Self {
            _cli: cli::State::new(event_queue_in),
            _clocks: clocks::State::new(event_queue_in),
            _filesystem: filesystem::State::new(event_queue_in),
            _http: http::State::new(event_queue_in),
            _io: io::State::new(event_queue_in),
            _random: random::State::new(event_queue_in),
        }
    }
}
