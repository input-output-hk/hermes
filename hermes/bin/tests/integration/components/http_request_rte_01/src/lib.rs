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

use std::fs;

use bindings::{
    exports::hermes::http_gateway::event::{Headers, HttpResponse},
    hermes::{
        binary::api::Bstr,
        cardano::api::{BlockSrc, CardanoBlock, CardanoBlockchainId, CardanoTxn},
        cron::api::CronTagged,
        ipfs::api::PubsubMessage,
    },
    wasi::http::types::{IncomingRequest, ResponseOutparam},
};
use url::Url;

use crate::bindings::hermes::http_request::api::Payload;

const REQUEST_ID: Option<u64> = Some(42);

struct HttpRequestApp;

impl bindings::exports::hermes::ipfs::event::Guest for HttpRequestApp {
    fn on_topic(_message: PubsubMessage) -> bool {
        true
    }
}

impl bindings::exports::hermes::cardano::event_on_block::Guest for HttpRequestApp {
    fn on_cardano_block(_blockchain: CardanoBlockchainId, _block: CardanoBlock, _source: BlockSrc) {
    }
}

impl bindings::exports::hermes::cardano::event_on_rollback::Guest for HttpRequestApp {
    fn on_cardano_rollback(_blockchain: CardanoBlockchainId, _slot: u64) {}
}

impl bindings::exports::hermes::cardano::event_on_txn::Guest for HttpRequestApp {
    fn on_cardano_txn(
        _blockchain: CardanoBlockchainId, _slot: u64, _txn_index: u32, _txn: CardanoTxn,
    ) {
    }
}

impl bindings::exports::hermes::cron::event::Guest for HttpRequestApp {
    fn on_cron(_event: CronTagged, _last: bool) -> bool {
        false
    }
}

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

// TOOD[RC]: Handle errors gracefully.
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

fn assert_eq<T: PartialEq + std::fmt::Debug>(left: T, right: T) {
    if left != right {
        test_log(&format!("{left:?} != {right:?}"));
        bindings::hermes::init::api::done(1);
    }
}

impl bindings::exports::hermes::http_request::event::Guest for HttpRequestApp {
    fn on_http_response(request_id: Option<u64>, response: Vec<u8>) {
        test_log(&format!(
            "got response with request_id={request_id:?}: {}",
            String::from_utf8(response.clone()).expect("should be valid UTF-8"),
        ));
        assert_eq(request_id, REQUEST_ID);
        bindings::hermes::init::api::done(0);
    }
}

impl bindings::exports::hermes::http_gateway::event::Guest for HttpRequestApp {
    fn reply(
        _body: Bstr, _headers: Headers, _path: String, _method: String,
    ) -> Option<HttpResponse> {
        None
    }
}

impl bindings::exports::hermes::kv_store::event::Guest for HttpRequestApp {
    fn kv_update(_key: String, _value: bindings::exports::hermes::kv_store::event::KvValues) {}
}

impl bindings::exports::wasi::http::incoming_handler::Guest for HttpRequestApp {
    fn handle(_request: IncomingRequest, _response_out: ResponseOutparam) {}
}

impl bindings::exports::hermes::integration_test::event::Guest for HttpRequestApp {
    fn test(
        _test: u32, _run: bool,
    ) -> Option<bindings::exports::hermes::integration_test::event::TestResult> {
        None
    }

    fn bench(
        _test: u32, _run: bool,
    ) -> Option<bindings::exports::hermes::integration_test::event::TestResult> {
        None
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
