//! Host - WASI - Random implementations

use crate::{event_queue::HermesEventQueueIn, runtime_extensions::state::Stateful};

pub(crate) mod insecure;
pub(crate) mod insecure_seed;
pub(crate) mod secure;

/// WASI State
pub(crate) struct State {
    /// insecure State
    _insecure: insecure::State,
    /// insecure_seed State
    _insecure_seed: insecure_seed::State,
    /// secure State
    _secure: secure::State,
}

impl Stateful for State {
    fn new(event_queue_in: &HermesEventQueueIn) -> Self {
        Self {
            _insecure: insecure::State::new(event_queue_in),
            _insecure_seed: insecure_seed::State::new(event_queue_in),
            _secure: secure::State::new(event_queue_in),
        }
    }
}
