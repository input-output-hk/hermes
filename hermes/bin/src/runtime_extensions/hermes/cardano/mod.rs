//! Cardano Blockchain runtime extension implementation.

use crate::runtime_extensions::state::{Context, Stateful};

mod event;
mod host;

/// State
pub(crate) struct State {}

impl Stateful for State {
    fn new(_ctx: &Context) -> Self {
        State {}
    }
}
