//! Host - WASI - Random implementations
#![allow(unused_variables)]

pub(crate) mod insecure;
pub(crate) mod insecure_seed;
pub(crate) mod secure;

use crate::runtime::extensions::Stateful;

#[allow(dead_code)]
/// WASI State
pub(crate) struct State {
    /// insecure State
    insecure: insecure::State,
    /// insecure_seed State
    insecure_seed: insecure_seed::State,
    /// secure State
    secure: secure::State,
}

impl Stateful for State {
    fn new(ctx: &crate::wasm::context::Context) -> Self {
        Self {
            insecure: insecure::State::new(ctx),
            insecure_seed: insecure_seed::State::new(ctx),
            secure: secure::State::new(ctx),
        }
    }
}
