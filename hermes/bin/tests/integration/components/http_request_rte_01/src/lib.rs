#![allow(unused)]

mod bindings {
    #![allow(clippy::missing_safety_doc)]

    wit_bindgen::generate!({
        world: "hermes",
        path: "../../../../../../wasm/wasi/wit",
        generate_all,
    });
}

use std::{
    error::Error,
    fs,
    thread::{self, sleep},
};

use bindings::{
    exports::hermes::http_gateway::event::{Headers, HttpResponse},
    hermes::{
        binary::api::Bstr,
        cardano::api::{BlockSrc, CardanoBlock, CardanoBlockchainId, CardanoTxn},
        cron::api::CronTagged,
        ipfs::api::PubsubMessage,
    },
    wasi::{
        clocks,
        http::types::{IncomingRequest, ResponseOutparam},
        random::random::get_random_u64,
    },
};
use serde::{Deserialize, Serialize};
use url::Url;

use crate::bindings::{hermes::http_request::api::Payload, wasi::clocks::wall_clock};

const REQUEST_ID: Option<u64> = Some(42);

struct HelloWorldModule;

impl bindings::exports::hermes::ipfs::event::Guest for HelloWorldModule {
    #[doc = r" Triggers when a message is received on a topic."]
    fn on_topic(message: PubsubMessage) -> bool {
        true
    }
}

impl bindings::exports::hermes::cardano::event_on_block::Guest for HelloWorldModule {
    fn on_cardano_block(blockchain: CardanoBlockchainId, block: CardanoBlock, source: BlockSrc) {}
}

impl bindings::exports::hermes::cardano::event_on_rollback::Guest for HelloWorldModule {
    fn on_cardano_rollback(blockchain: CardanoBlockchainId, slot: u64) {}
}

impl bindings::exports::hermes::cardano::event_on_txn::Guest for HelloWorldModule {
    fn on_cardano_txn(blockchain: CardanoBlockchainId, slot: u64, txn_index: u32, txn: CardanoTxn) {
    }
}

impl bindings::exports::hermes::cron::event::Guest for HelloWorldModule {
    fn on_cron(event: CronTagged, last: bool) -> bool {
        false
    }
}

// TODO[RC]: Rename hello world
impl bindings::exports::hermes::init::event::Guest for HelloWorldModule {
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

        let payload = make_payload(&http_server.to_string());
        test_log(&format!("sending request"));
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

    let request_body = make_body(&host_uri);

    let payload = Payload {
        host_uri: format!("{scheme}://{host_uri}"),
        port,
        body: request_body.to_vec(),
        request_id: REQUEST_ID,
    };
    payload
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

impl bindings::exports::hermes::http_request::event::Guest for HelloWorldModule {
    fn on_http_response(request_id: Option<u64>, response: Vec<u8>) {
        test_log(&format!(
            "got response with request_id={request_id:?}: {}",
            String::from_utf8(response.clone()).unwrap(),
        ));
        assert_eq!(request_id, REQUEST_ID);
    }
}

/// response should be option
impl bindings::exports::hermes::http_gateway::event::Guest for HelloWorldModule {
    fn reply(body: Bstr, headers: Headers, path: String, method: String) -> Option<HttpResponse> {
        Some(HttpResponse {
            code: 200,
            headers,
            body,
        })
    }
}

impl bindings::exports::hermes::kv_store::event::Guest for HelloWorldModule {
    fn kv_update(key: String, value: bindings::exports::hermes::kv_store::event::KvValues) {
        bindings::hermes::logging::api::log(
            bindings::hermes::logging::api::Level::Trace,
            None,
            None,
            None,
            None,
            None,
            format!("FROM WASM MODULE: kv {}", key).as_str(),
            None,
        );
    }
}

impl bindings::exports::wasi::http::incoming_handler::Guest for HelloWorldModule {
    fn handle(request: IncomingRequest, response_out: ResponseOutparam) {}
}

impl bindings::exports::hermes::integration_test::event::Guest for HelloWorldModule {
    fn test(
        test: u32, run: bool,
    ) -> Option<bindings::exports::hermes::integration_test::event::TestResult> {
        None
    }

    fn bench(
        test: u32, run: bool,
    ) -> Option<bindings::exports::hermes::integration_test::event::TestResult> {
        None
    }
}

bindings::export!(HelloWorldModule with_types_in bindings);

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
