//! Hash runtime extension implementation.

mod blake2b;
mod host;

/// State
pub(crate) struct State {}

impl State {
    ///
    #[allow(dead_code)]
    fn new() -> Self {
        State {}
    }
}
