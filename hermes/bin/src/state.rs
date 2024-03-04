//! Hermes state implementation.

use crate::runtime_extensions::{
    hermes,
    state::{Context, Stateful},
    wasi,
};

#[allow(dead_code)]
/// State for Hermes runtime
pub(crate) struct HermesState {
    /// Hermes custom extensions state
    pub hermes: hermes::State,

    /// WASI standard extensions state
    pub wasi: wasi::State,

    /// The context of the wasm modules using this State.
    pub ctx: Context,
}

impl Stateful for HermesState {
    fn new(ctx: &Context) -> HermesState {
        HermesState {
            hermes: hermes::State::new(ctx),
            wasi: wasi::State::new(ctx),
            ctx: ctx.clone(),
        }
    }
}
