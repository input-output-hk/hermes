//! Host - WASI - Clock implementations
//!
#![allow(unused_variables)]

use crate::runtime::extensions::NewState;

mod monotonic;
mod wall;

/// WASI State
pub(crate) struct State {
    /// monotonic State
    monotonic: monotonic::State,
    /// wall State
    wall: wall::State,
}

impl NewState for State {
    fn new(ctx: &crate::wasm::context::Context) -> Self {
        Self {
            monotonic: monotonic::State::new(ctx),
            wall: wall::State::new(ctx),
        }
    }
}
