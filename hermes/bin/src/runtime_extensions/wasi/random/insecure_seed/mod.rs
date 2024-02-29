//! Insecure RNG seed runtime extension implementation.

mod host;

/// WASI State
pub(crate) struct State {}

impl State {
    ///
    #[allow(dead_code)]
    fn new() -> Self {
        Self {}
    }
}
