//! Hermes state implementation.

use std::sync::Arc;

use crate::{runtime_extensions::state::State, wasm::context::Context};

#[allow(dead_code)]
/// State for Hermes runtime
pub(crate) struct HermesState {
    /// Runtime extensions state
    pub(crate) state: Arc<State>,

    /// The context of the wasm modules using this State.
    pub(crate) ctx: Context,
}

impl HermesState {
    /// Creates a new instance of the `HermesState`.
    pub(crate) fn new(ctx: Context, state: Arc<State>) -> HermesState {
        Self { state, ctx }
    }
}
