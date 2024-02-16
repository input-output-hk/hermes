//! Logging runtime extension implementation.

use crate::{event_queue::HermesEventQueueIn, runtime_extensions::state::Stateful};

mod host;

/// State
pub(crate) struct State {}

impl Stateful for State {
    fn new(_ctx: &HermesEventQueueIn) -> Self {
        State {}
    }
}
