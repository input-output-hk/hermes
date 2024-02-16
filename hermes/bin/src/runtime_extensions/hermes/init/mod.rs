//! Init runtime extension implementation.

use crate::{event_queue::HermesEventQueueIn, runtime_extensions::state::Stateful};

mod event;

/// State
pub(crate) struct State {}

impl Stateful for State {
    fn new(event_queue_in: &HermesEventQueueIn) -> Self {
        let event_queue_in = event_queue_in.clone();
        event_queue_in.add(Box::new(event::InitEvent {}));
        State {}
    }
}
