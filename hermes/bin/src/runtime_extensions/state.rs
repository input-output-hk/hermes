//! Hermes runtime extensions state.

use super::{hermes, wasi};
use crate::event_queue::HermesEventQueueIn;

/// All Hermes runtime extensions states need to implement this.
pub(crate) trait Stateful: Send + Sync {
    /// Initial state for the given context
    fn new(event_queue_in: &HermesEventQueueIn) -> Self;
}

/// All runtime extensions state
pub(crate) struct State {
    /// Hermes custom extensions state
    pub(crate) hermes: hermes::State,

    /// WASI standard extensions state
    pub(crate) _wasi: wasi::State,
}

impl Stateful for State {
    fn new(event_queue_in: &HermesEventQueueIn) -> Self {
        Self {
            hermes: hermes::State::new(event_queue_in),
            _wasi: wasi::State::new(event_queue_in),
        }
    }
}
