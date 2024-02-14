//! Host - JSON implementations

use crate::{
    runtime_extensions::{
        bindings::hermes::json::api::Host,
        state::{Context, Stateful},
    },
    state::HermesState,
};

/// State
pub(crate) struct State {}

impl Stateful for State {
    fn new(_ctx: &Context) -> Self {
        State {}
    }
}

impl Host for HermesState {}
