//! Hermes runtime extensions state.

use rusty_ulid::Ulid;

use super::{hermes, wasi};

/// All Hermes runtime extensions states need to implement this.
pub(crate) trait Stateful {
    /// Initial state for the given context
    fn new(ctx: &Context) -> Self;
}

/// All runtime extensions state
pub(crate) struct State {
    /// Hermes custom extensions state
    _hermes: hermes::State,

    /// WASI standard extensions state
    _wasi: wasi::State,
}

impl Stateful for State {
    fn new(ctx: &Context) -> Self {
        Self {
            _hermes: hermes::State::new(ctx),
            _wasi: wasi::State::new(ctx),
        }
    }
}

/// A WASM module's context structure, which is intended to be passed to the
/// `wasmtime::Store` during the WASM module's state initialization process.
#[derive(Clone)]
pub(crate) struct Context {
    /// Hermes application name
    app_name: String,

    /// module ULID id
    module_id: Ulid,

    /// event name to be executed
    event_name: Option<String>,

    /// module's execution counter
    counter: u64,
}

impl Context {
    /// Creates a new instance of the `Context`.
    pub(crate) fn new(app_name: String) -> Self {
        Self {
            app_name,
            module_id: Ulid::generate(),
            event_name: None,
            counter: 0,
        }
    }

    /// Increments the module's execution counter and sets the event name to be executed
    pub(crate) fn use_for(&mut self, event_name: String) {
        self.event_name = Some(event_name);
        self.counter += 1;
    }

    /// Get the application name
    #[allow(dead_code)]
    pub(crate) fn app_name(&self) -> &str {
        &self.app_name
    }

    /// Get the module id
    #[allow(dead_code)]
    pub(crate) fn module_id(&self) -> &Ulid {
        &self.module_id
    }

    /// Get the event name
    #[allow(dead_code)]
    pub(crate) fn event_name(&self) -> Option<&String> {
        self.event_name.as_ref()
    }

    /// Get the counter value
    #[allow(dead_code)]
    pub(crate) fn counter(&self) -> u64 {
        self.counter
    }
}
