//! Host - WASI - Clock implementations
#![allow(unused_variables)]

use crate::runtime::extensions::Stateful;

mod monotonic;
mod wall;

#[allow(dead_code)]
/// WASI State
pub(crate) struct State {
    /// monotonic State
    monotonic: monotonic::State,
    /// wall State
    wall: wall::State,
}

impl Stateful for State {
    fn new(ctx: &crate::state::Context) -> Self {
        Self {
            monotonic: monotonic::State::new(ctx),
            wall: wall::State::new(ctx),
        }
    }
}
