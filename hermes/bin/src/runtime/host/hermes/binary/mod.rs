//! Host - CBOR implementations
//!
#![allow(unused_variables)]

use crate::runtime::extensions::hermes::binary::api::Host;
use crate::runtime::extensions::{HermesState, NewState};

/// State
pub(crate) struct State {}

impl NewState for State {
    fn new(_ctx: &crate::wasm::context::Context) -> Self {
        State {}
    }
}

impl Host for HermesState {}
