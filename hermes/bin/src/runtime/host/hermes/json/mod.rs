//! Host - JSON implementations
#![allow(unused_variables)]

use crate::runtime::extensions::{hermes::json::api::Host, HermesState, Stateful};

/// State
pub(crate) struct State {}

impl Stateful for State {
    fn new(_ctx: &crate::state::Context) -> Self {
        State {}
    }
}

impl Host for HermesState {}
