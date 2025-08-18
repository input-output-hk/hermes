//! The test Hermes App.
#![allow(
    clippy::missing_safety_doc,
    clippy::missing_docs_in_private_items,
    clippy::expect_used
)]

mod bindings {

    wit_bindgen::generate!({
        world: "hermes",
        path: "../../../../../../wasm/wasi/wit",
        generate_all,
    });
}
mod stub;
mod utils;

use std::{fs, io::Write as _};

use crate::utils::{busy_wait_s, make_payload, test_log};

const REQUEST_COUNT: usize = 5;
const RESPONSES_FILE: &str = "responses.txt";
const CONTENT: &[u8] = b"\xF0\x9F\xA6\x80\n";
const WAIT_FOR_SECS: u64 = 1;

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

        std::fs::File::create(RESPONSES_FILE).expect("failed to create file");

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
        std::fs::File::options()
            .append(true)
            .open(RESPONSES_FILE)
            .expect("failed to open file")
            .write_all(CONTENT)
            .expect("failed to write content to file");

        if std::fs::read(RESPONSES_FILE)
            .expect("failed to read file")
            .len()
            == CONTENT
                .len()
                .checked_mul(REQUEST_COUNT)
                .expect("multiplication overflowed")
        {
            test_log(&format!(
                "Reached {REQUEST_COUNT} responses, calling done()",
            ));
            bindings::hermes::init::api::done(0);
        }
    }
}

bindings::export!(HttpRequestApp with_types_in bindings);
