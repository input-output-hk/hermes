//! WASM component initialization export.

use crate::runtime_extensions::bindings::unchecked_exports;

unchecked_exports::define! {
    /// Extends [`wasmtime::component::Instance`] with guest functions for init event.
    pub(crate) trait ComponentInstanceExt {
         #[wit("hermes:init/event", "init")]
        fn hermes_init_event_init() -> bool;
    }
}
