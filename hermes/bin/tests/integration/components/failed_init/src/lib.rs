//! The test Hermes App.
#![allow(
    clippy::missing_safety_doc,
    clippy::missing_docs_in_private_items,
    clippy::expect_used
)]

mod bindings {

    wit_bindgen::generate!({
        world: "hermes:app/hermes",
        path: "../../../../../../wasm/wasi/wit",
        inline: "
            package hermes:app;

            world hermes {
                import hermes:logging/api;
                
                export hermes:init/event;
            }
        ",
        generate_all,
    });
}

struct FailedInitApp;

impl bindings::exports::hermes::init::event::Guest for FailedInitApp {
    fn init() -> bool {
        test_log("module explicitly failing to initialize");
        false
    }
}

bindings::export!(FailedInitApp with_types_in bindings);

fn test_log(s: &str) {
    bindings::hermes::logging::api::log(
        bindings::hermes::logging::api::Level::Trace,
        None,
        None,
        None,
        None,
        None,
        format!("[TEST] {s}").as_str(),
        None,
    );
}
