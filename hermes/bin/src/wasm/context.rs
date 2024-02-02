//! WASM host context implementation.

use rusty_ulid::Ulid;

/// A WASM host context structure, which is intended to be passed to the `wasmtime::Store`
/// during the WASM state initialization process.
#[derive(Clone)]
pub(crate) struct Context {
    /// Hermes application name
    app_name: String,

    /// module ULID id
    module_id: Ulid,

    ///
    event_name: Option<String>,

    ///
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

    ///
    pub(crate) fn use_for(&mut self, even_name: String) {
        self.event_name = Some(even_name);
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
