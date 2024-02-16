//! Hermes state implementation.

use crate::runtime_extensions::state::{Context, State, Stateful};

#[allow(dead_code)]
/// State for Hermes runtime
pub(crate) struct HermesState {
    /// Runtime extensions state
    pub(crate) state: State,

    /// The context of the wasm modules using this State.
    pub(crate) ctx: Context,
}

impl HermesState {
    /// Creates a new instance of the `HermesState`.
    pub(crate) fn new(ctx: Context) -> HermesState {
        Self {
            state: State::new(&ctx),
            ctx,
        }
    }
}
