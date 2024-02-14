//! Host - WASI - Random implementations

use crate::runtime_extensions::state::{Context, Stateful};

pub(crate) mod insecure;
pub(crate) mod insecure_seed;
pub(crate) mod secure;

/// WASI State
pub(crate) struct State {
    /// insecure State
    _insecure: insecure::State,
    /// insecure_seed State
    _insecure_seed: insecure_seed::State,
    /// secure State
    _secure: secure::State,
}

impl Stateful for State {
    fn new(ctx: &Context) -> Self {
        Self {
            _insecure: insecure::State::new(ctx),
            _insecure_seed: insecure_seed::State::new(ctx),
            _secure: secure::State::new(ctx),
        }
    }
}
