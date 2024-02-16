//! IO Error runtime extension implementation.

use crate::{event_queue::HermesEventQueueIn, runtime_extensions::state::Stateful};

mod host;

/// WASI State
pub(crate) struct State {}

impl Stateful for State {
    fn new(_event_queue_in: &HermesEventQueueIn) -> Self {
        Self {}
    }
}
