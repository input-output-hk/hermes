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
                import wasi:clocks/monotonic-clock@0.2.6;
                import hermes:sqlite/api;
                import hermes:logging/api;
                import hermes:init/api;
                import hermes:http-request/api;
                
                export hermes:init/event;
                export hermes:http-request/event;
            }
        ",
        generate_all,
    });
}

mod utils;

use std::fs;

use crate::utils::{busy_wait_s, make_payload, test_log};

const REQUEST_COUNT: usize = 5;
const WAIT_FOR_SECS: u64 = 5;

struct HttpRequestApp;

impl bindings::exports::hermes::init::event::Guest for HttpRequestApp {
    fn init() -> bool {
        test_log("init sleep component");
        let settings_json = fs::read_to_string("/lib/sleep_module/settings.schema.json")
            .expect("cannot read settings file");
        let parsed_json: serde_json::Value =
            serde_json::from_str(&settings_json).expect("unable to parse settings as JSON");
        let http_server = parsed_json
            .get("http_server")
            .expect("missing http_server in settings")
            .as_str()
            .expect("http_server is not a string");

        let sqlite =
            bindings::hermes::sqlite::api::open(false, false).expect("failed to connect to db");
        sqlite
            .execute(
                r"
                    CREATE TABLE IF NOT EXISTS counter (
                        value INTEGER
                    );
                    ",
            )
            .expect("failed to create DB");
        sqlite
            .execute("INSERT INTO counter(value) VALUES(0);")
            .expect("failed to insert counter");
        sqlite.close().expect("failed to close connection");

        for i in 0..REQUEST_COUNT
            .try_into()
            .expect("failed to convert request count to usize")
        {
            let payload = make_payload(http_server, Some(i));
            test_log(&format!("sending sleep app request {i}"));
            let send_result = bindings::hermes::http_request::api::send(&payload);
            test_log(&format!(
                "request sent (result={send_result:?}), awaiting response"
            ));
        }

        true
    }
}

impl bindings::exports::hermes::http_request::event::Guest for HttpRequestApp {
    fn on_http_response(
        request_id: Option<u64>,
        response: Vec<u8>,
    ) {
        test_log(&format!(
            "got response with request_id={request_id:?}: {}",
            String::from_utf8(response).expect("should be valid UTF-8")
        ));
        busy_wait_s(WAIT_FOR_SECS);

        test_log(&format!("sqlite open request_id={request_id:?}"));
        let sqlite = bindings::hermes::sqlite::api::open(false, false)
            .inspect_err(|err| {
                test_log(&format!(
                    "open failed with request_id={request_id:?} and with: {err:?}"
                ));
            })
            .expect("failed to connect to db");
        test_log(&format!("sqlite prepare request_id={request_id:?}"));
        let prep = sqlite
            .prepare(
                r"
                    UPDATE counter
                    SET value = value + 1
                    RETURNING value;
                ",
            )
            .inspect_err(|err| {
                test_log(&format!(
                    "prepare failed with request_id={request_id:?} and with: {err:?}"
                ));
            })
            .expect("failed to prepare statement");

        test_log(&format!("sqlite step request_id={request_id:?}"));
        prep.step()
            .inspect_err(|err| {
                test_log(&format!(
                    "step failed request_id={request_id:?} and with: {err:?}"
                ));
            })
            .expect("failed to make step");

        test_log(&format!("sqlite column request_id={request_id:?}"));
        let bindings::hermes::sqlite::api::Value::Int32(value) =
            prep.column(0).expect("failed to get value")
        else {
            panic!("unexpected type!!!");
        };

        test_log(&format!("sqlite finalized request_id={request_id:?}"));
        prep.finalize().expect("failed to finalize statement");

        test_log(&format!("sqlite close request_id={request_id:?}"));
        sqlite.close().expect("failed to close connection");

        let current_count: usize = value.try_into().expect("failed to convert i32 to usize");
        if current_count == REQUEST_COUNT {
            test_log(&format!(
                "All {REQUEST_COUNT} responses written correctly, calling done()",
            ));
            bindings::hermes::init::api::done(0);
        }
    }
}

bindings::export!(HttpRequestApp with_types_in bindings);
