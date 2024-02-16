//! Hermes runtime extensions state.

use super::{hermes, wasi};
use crate::event_queue::HermesEventQueueIn;

/// All Hermes runtime extensions states need to implement this.
pub(crate) trait Stateful {
    /// Initial state for the given context
    fn new(event_eueue_in: &HermesEventQueueIn) -> Self;
}

/// All runtime extensions state
pub(crate) struct State {
    /// Hermes custom extensions state
    _hermes: hermes::State,

    /// WASI standard extensions state
    _wasi: wasi::State,
}

impl Stateful for State {
    fn new(event_eueue_in: &HermesEventQueueIn) -> Self {
        Self {
            _hermes: hermes::State::new(event_eueue_in),
            _wasi: wasi::State::new(event_eueue_in),
        }
    }
}
