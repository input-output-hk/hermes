#![allow(missing_docs)]
//! Staked ADA Indexer Mock Component

shared::bindings_generate!({
    world: "hermes:app/hermes",
    path: "../../../../../../wasm/wasi/wit",
    inline: "
        package hermes:app;

        world hermes {
            include wasi:cli/imports@0.2.6;
            import hermes:cardano/api;
            import hermes:logging/api;
            import hermes:init/api;
            import hermes:sqlite/api;

            export hermes:init/event;
        }
    ",
    share: ["hermes:sqlite", "hermes:logging"],
});

export!(Component);

mod events;

use shared::utils::log::{self, error};

/// Mocked indexer component.
struct Component;

impl exports::hermes::init::event::Guest for Component {
    fn init() -> bool {
        log::init(log::LevelFilter::Trace);
        match events::init() {
            Ok(()) => true,
            Err(error) => {
                error!(target: "staked_ada_indexer::init", error:?; "Not handled");
                false
            },
        }
    }
}
