//! Hermes runtime extensions state.

use super::{hermes, wasi};

/// All Hermes runtime extensions states need to implement this.
pub(crate) trait Stateful: Send + Sync {
    /// Initial state for the given context
    fn new() -> Self;
}

/// All runtime extensions state
pub(crate) struct State {
    /// Hermes custom extensions state
    pub(crate) hermes: hermes::State,

    /// WASI standard extensions state
    pub(crate) _wasi: wasi::State,
}

impl Stateful for State {
    fn new() -> Self {
        Self {
            hermes: hermes::State::new(),
            _wasi: wasi::State::new(),
        }
    }
}
