//! Host - WASI - Clock implementations

use crate::state::Stateful;

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
    fn new(ctx: &crate::state::Context) -> Self {
        Self {
            _monotonic: monotonic::State::new(ctx),
            _wall: wall::State::new(ctx),
        }
    }
}
