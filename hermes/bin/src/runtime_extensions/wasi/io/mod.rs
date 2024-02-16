//! Host - WASI IO Implementation

use crate::{event_queue::HermesEventQueueIn, runtime_extensions::state::Stateful};

pub(crate) mod error;
pub(crate) mod streams;

/// WASI State
pub(crate) struct State {
    /// WASI IO error state
    _error: error::State,
    /// WASI IO streams state
    _streams: streams::State,
}

impl Stateful for State {
    fn new(event_queue_in: &HermesEventQueueIn) -> Self {
        Self {
            _error: error::State::new(event_queue_in),
            _streams: streams::State::new(event_queue_in),
        }
    }
}
