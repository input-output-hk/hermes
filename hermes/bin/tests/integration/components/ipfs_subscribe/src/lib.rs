//! The test Hermes App.
#![allow(
    clippy::missing_safety_doc,
    clippy::missing_docs_in_private_items,
    clippy::expect_used,
    clippy::panic
)]

mod bindings {

    wit_bindgen::generate!({
        world: "hermes:app/hermes",
        path: "../../../../../../wasm/wasi/wit",
        inline: "
            package hermes:app;

            world hermes {
                include wasi:cli/imports@0.2.6;

                import wasi:clocks/monotonic-clock@0.2.6;
                import hermes:logging/api;
                import hermes:init/api;
                import hermes:doc-sync/api;

                export hermes:init/event;
            }
        ",
        generate_all,
    });
}

struct IPFSSubscribeApp;

impl bindings::exports::hermes::init::event::Guest for IPFSSubscribeApp {
    fn init() -> bool {
        bindings::hermes::doc_sync::api::SyncChannel::new("ipfs_channel");
        false
    }
}

bindings::export!(IPFSSubscribeApp with_types_in bindings);
