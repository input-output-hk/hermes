//! Host - WASI - Clock implementations

use crate::runtime_extensions::state::Stateful;

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
    fn new() -> Self {
        Self {
            _monotonic: monotonic::State::new(),
            _wall: wall::State::new(),
        }
    }
}
