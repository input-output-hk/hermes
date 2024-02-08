//! Host - CBOR implementations

use crate::{
    runtime::extensions::bindings::hermes::binary::api::Host,
    state::{HermesState, Stateful},
};

/// State
pub(crate) struct State {}

impl Stateful for State {
    fn new(_ctx: &crate::state::Context) -> Self {
        State {}
    }
}

impl Host for HermesState {}
