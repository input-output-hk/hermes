//! Simple component that checks Sqlite version to be major 3.

wit_bindgen::generate!({
    world: "hermes:app/hermes",
    path: "../../../wasi/wit",
    inline: "
        package hermes:app;

        world hermes {
            include wasi:cli/imports@0.2.6;
            import hermes:sqlite/api;
            import hermes:logging/api;
            import hermes:init/api;
            
            export hermes:init/event;
        }
    ",
    generate_all,
});

export!(TestComponent);

use crate::hermes::{init::api::done, sqlite};

struct TestComponent;

fn simple_log(msg: &str) {
    hermes::logging::api::log(
        hermes::logging::api::Level::Trace,
        None,
        None,
        None,
        None,
        None,
        msg,
        None,
    );
}

impl exports::hermes::init::event::Guest for TestComponent {
    fn init() -> bool {
        simple_log("🍊 Init event trigger");

        let has_major_3_version = sqlite::api::open(false, true)
            .and_then(|conn| {
                conn.prepare("SELECT sqlite_version()")
                    .and_then(|stmt| stmt.step().and_then(|_| stmt.column(0)))
            })
            .inspect_err(|err| simple_log(&err.to_string()))
            .is_ok_and(|val| matches!(val, sqlite::api::Value::Text(s) if s.starts_with("3")));

        if has_major_3_version {
            simple_log("Success – version 3.x.x");
            done(0);
            true
        } else {
            simple_log("Failed to verify 3.x.x version");
            done(1);
            false
        }
    }
}
