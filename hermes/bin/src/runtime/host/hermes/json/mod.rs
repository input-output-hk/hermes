//! Host - JSON implementations
#![allow(unused_variables)]

use crate::runtime::extensions::{hermes::json::api::Host, HermesState, NewState};

/// State
pub(crate) struct State {}

impl NewState for State {
    fn new(_ctx: &crate::wasm::context::Context) -> Self {
        State {}
    }
}

impl Host for HermesState {}
