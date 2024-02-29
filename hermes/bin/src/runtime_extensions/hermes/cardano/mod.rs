//! Cardano Blockchain runtime extension implementation.

mod event;
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
