//! Staked ADA Indexing Module

shared::bindings_generate!({
    world: "hermes:app/hermes",
    path: "../../../../../wasm/wasi/wit",
    inline: "
        package hermes:app;

        world hermes {
            include wasi:cli/imports@0.2.6;
            import hermes:cardano/api;
            import hermes:logging/api;
            import hermes:init/api;
            import hermes:sqlite/api;
            
            export hermes:init/event;
            // export hermes:cardano/event-on-block;
        }
    ",
    share: ["hermes:cardano", "hermes:sqlite", "hermes:logging"],
});

#[allow(unused)]
mod database;

// use self::

export!(StakedAdaComponent);

struct StakedAdaComponent;

impl exports::hermes::init::event::Guest for StakedAdaComponent {
    fn init() -> bool {
        true
    }
}
