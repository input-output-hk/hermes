//! Runtime modules - extensions
//!
//! *Note*
//! Inspect the generated code with:
//! ```
//! cargo expand --bin hermes runtime::extensions
//! ```
#![allow(clippy::indexing_slicing)]

use crate::runtime;
use crate::wasm::context::Context;
use wasmtime::{
    component::{bindgen, Linker},
    Engine,
};

bindgen!({
    world: "hermes",
    path: "../../wasm/wasi/wit",
});

/// All Hermes extensions states need to implement this.
pub(crate) trait NewState {
    /// Initial state for the given context
    fn new(ctx: &Context) -> Self;
}

#[allow(dead_code)]
/// State for Hermes runtime
pub(crate) struct HermesState {
    /// Hermes custom extensions state
    pub hermes: runtime::host::hermes::State,

    /// WASI standard extensions state
    pub wasi: runtime::host::wasi::State,

    /// The context of the wasm modules using this State.
    pub ctx: Context,
}

impl NewState for HermesState {
    fn new(ctx: &Context) -> HermesState {
        HermesState {
            hermes: runtime::host::hermes::State::new(ctx),
            wasi: runtime::host::wasi::State::new(ctx),
            ctx: ctx.clone(),
        }
    }
}

#[allow(dead_code)]
/// Link a component to the Hermes runtime.
pub(crate) fn link_runtime(
    engine: &Engine,
) -> Result<Linker<HermesState>, Box<dyn std::error::Error>> {
    let mut linker = Linker::new(engine);
    Hermes::add_to_linker(&mut linker, |state: &mut HermesState| state)?;

    Ok(linker)
}
