//! Host - WASI - Clock implementations

use crate::{event_queue::HermesEventQueueIn, runtime_extensions::state::Stateful};

mod monotonic;
mod wall;

/// WASI State
pub(crate) struct State {
    /// monotonic State
    _monotonic: monotonic::State,
    /// wall State
    _wall: wall::State,
}

impl Stateful for State {
    fn new(_event_queue_in: &HermesEventQueueIn) -> Self {
        Self {
            _monotonic: monotonic::State::new(_event_queue_in),
            _wall: wall::State::new(_event_queue_in),
        }
    }
}
