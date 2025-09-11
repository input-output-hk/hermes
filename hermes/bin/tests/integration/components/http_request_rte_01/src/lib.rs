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
                include wasi:cli/imports@0.2.6;

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

use std::fs;

use url::Url;

use crate::bindings::hermes::http_request::api::Payload;

const REQUEST_ID: Option<u64> = Some(42);

struct HttpRequestApp;

impl bindings::exports::hermes::init::event::Guest for HttpRequestApp {
    fn init() -> bool {
        let settings_json = fs::read_to_string("/lib/test_module/settings.schema.json")
            .expect("cannot read settings file");
        let parsed_json: serde_json::Value =
            serde_json::from_str(&settings_json).expect("unable to parse settings as JSON");
        let http_server = parsed_json
            .get("http_server")
            .expect("missing http_server in settings")
            .as_str()
            .expect("http_server is not a string");

        let payload = make_payload(http_server);
        test_log("sending request");
        let send_result = bindings::hermes::http_request::api::send(&payload);
        test_log(&format!(
            "request sent (result={send_result:?}), awaiting response"
        ));
        true
    }
}

// TODO[RC]: Handle errors gracefully.
fn make_payload(http_server: &str) -> Payload {
    test_log(&format!("parsing addr: {http_server}"));

    let parsed = Url::parse(http_server).expect("invalid URL");
    let scheme = parsed.scheme();
    let host_uri = parsed.host_str().expect("invalid host URI").to_string();
    let port = parsed.port_or_known_default().expect("invalid port");
    test_log(&format!(
        "parsed: scheme: {scheme}, host URI: {host_uri}, port: {port}"
    ));

    let body = make_body(&host_uri);

    Payload {
        host_uri: format!("{scheme}://{host_uri}"),
        port,
        body,
        request_id: REQUEST_ID,
    }
}

fn make_body(host_uri: &str) -> Vec<u8> {
    let request_body = format!(
        "GET /test.txt HTTP/1.1\r\n\
        Host: {host_uri}\r\n\
        Content-Type: application/json\r\n\
        Content-Length: 15\r\n\
        Connection: close\r\n\
        \r\n\
        {{\"key\":\"value\"}}"
    );
    test_log(&format!("request body: {request_body}"));
    request_body.into_bytes()
}

fn assert_eq<T: PartialEq + std::fmt::Debug>(
    left: T,
    right: T,
) {
    if left != right {
        test_log(&format!("{left:?} != {right:?}"));
        bindings::hermes::init::api::done(1);
    }
}

impl bindings::exports::hermes::http_request::event::Guest for HttpRequestApp {
    fn on_http_response(
        request_id: Option<u64>,
        response: Vec<u8>,
    ) {
        test_log(&format!(
            "got response with request_id={request_id:?}: {}",
            String::from_utf8(response.clone()).expect("should be valid UTF-8"),
        ));
        assert_eq(request_id, REQUEST_ID);
        bindings::hermes::init::api::done(0);
    }
}

bindings::export!(HttpRequestApp with_types_in bindings);

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
