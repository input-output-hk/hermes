//! Insecure RNG seed runtime extension implementation.

use crate::runtime_extensions::state::Stateful;

mod host;

/// WASI State
pub(crate) struct State {}

impl Stateful for State {
    fn new() -> Self {
        Self {}
    }
}
