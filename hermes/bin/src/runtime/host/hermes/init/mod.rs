//! Host - Init implementations
#![allow(unused_variables)]

use crate::runtime::extensions::NewState;

#[allow(dead_code)]

/// State
pub(crate) struct State {}

impl NewState for State {
    fn new(_ctx: &crate::wasm::context::Context) -> Self {
        State {}
    }
}
