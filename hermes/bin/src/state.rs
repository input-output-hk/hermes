//! Hermes state implementation.

use std::sync::Arc;

use rusty_ulid::Ulid;

use crate::runtime_extensions::state::State;

#[allow(dead_code)]
/// State for Hermes state
pub(crate) struct HermesState {
    /// Runtime extensions state
    pub(crate) state: Arc<State>,
    // /// The context of the wasm modules using this State.
    // pub(crate) ctx: Context,
}

impl HermesState {
    /// Creates a new instance of the `HermesState`.
    pub(crate) fn new(state: Arc<State>) -> HermesState {
        Self { state }
    }
}

/// A Hermes running context, which should be passed to the WASM runtime.
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
    pub(crate) fn _new(app_name: String) -> Self {
        Self {
            app_name,
            module_id: Ulid::generate(),
            event_name: None,
            counter: 0,
        }
    }

    /// Increments the module's execution counter and sets the event name to be executed
    pub(crate) fn _use_for(&mut self, event_name: String) {
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
