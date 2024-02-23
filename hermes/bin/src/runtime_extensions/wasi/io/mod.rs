//! Host - WASI IO Implementation

use crate::runtime_extensions::state::{Context, Stateful};

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
    fn new(ctx: &Context) -> Self {
        Self {
            _error: error::State::new(ctx),
            _streams: streams::State::new(ctx),
        }
    }
}
