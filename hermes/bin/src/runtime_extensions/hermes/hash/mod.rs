//! Hash runtime extension implementation.

use crate::runtime_extensions::state::Stateful;

mod blake2b;
mod host;

/// State
pub(crate) struct State {}

impl Stateful for State {
    fn new() -> Self {
        State {}
    }
}
