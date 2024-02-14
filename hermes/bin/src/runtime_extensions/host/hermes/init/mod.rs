//! Host - Init implementations

use crate::runtime_extensions::extensions::state::{Context, Stateful};

/// State
pub(crate) struct State {}

impl Stateful for State {
    fn new(_ctx: &Context) -> Self {
        State {}
    }
}
