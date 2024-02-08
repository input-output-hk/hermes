//! Host - Init implementations
#![allow(unused_variables)]

use crate::runtime::extensions::Stateful;

#[allow(dead_code)]

/// State
pub(crate) struct State {}

impl Stateful for State {
    fn new(_ctx: &crate::state::Context) -> Self {
        State {}
    }
}
