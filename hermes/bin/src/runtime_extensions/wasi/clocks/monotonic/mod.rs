//! Monotonic clock runtime extension implementation.

use crate::runtime_extensions::state::{Context, Stateful};

mod host;

/// WASI State
pub(crate) struct State {}

impl Stateful for State {
    fn new(_ctx: &Context) -> Self {
        Self {}
    }
}
