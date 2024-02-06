//! Host - WASI IO Implementation

use crate::runtime::extensions::Stateful;

pub(crate) mod error;
pub(crate) mod streams;

#[allow(dead_code)]
/// WASI State
pub(crate) struct State {
    /// WASI IO error state
    error: error::State,
    /// WASI IO streams state
    streams: streams::State,
}

impl Stateful for State {
    fn new(ctx: &crate::wasm::context::Context) -> Self {
        Self {
            error: error::State::new(ctx),
            streams: streams::State::new(ctx),
        }
    }
}
