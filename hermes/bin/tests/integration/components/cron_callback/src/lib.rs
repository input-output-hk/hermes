//! The test Hermes App.
#![allow(
    clippy::missing_safety_doc,
    clippy::missing_docs_in_private_items,
    clippy::expect_used
)]

use std::time::Duration;

mod bindings {
    wit_bindgen::generate!({
        world: "hermes:app/hermes",
        path: "../../../../../../wasm/wasi/wit",
        inline: "
            package hermes:app;

            world hermes {
                import hermes:cron/api;
                import hermes:logging/api;
                import hermes:init/api;
                
                export hermes:init/event;
                export hermes:cron/event;
            }
        ",
        generate_all,
    });
}

const CRON_TAG: &str = "100-millis";

struct CronCallbackApp;

impl bindings::exports::hermes::init::event::Guest for CronCallbackApp {
    fn init() -> bool {
        let result = bindings::hermes::cron::api::delay(
            Duration::from_millis(100).as_nanos() as u64,
            CRON_TAG,
        );

        test_log(&format!("cron event added with result={}", result));
        assert!(result);

        true
    }
}

impl bindings::exports::hermes::cron::event::Guest for CronCallbackApp {
    fn on_cron(
        event: bindings::exports::hermes::cron::event::CronTagged,
        last: bool,
    ) -> bool {
        test_log(&format!("got cron event with tag={}", event.tag));
        assert!(last);
        assert_eq!(event.tag, CRON_TAG);
        bindings::hermes::init::api::done(0);
        true
    }
}

bindings::export!(CronCallbackApp with_types_in bindings);

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
