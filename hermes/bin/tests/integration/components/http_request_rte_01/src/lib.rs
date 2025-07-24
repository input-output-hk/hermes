mod bindings {
    #![allow(clippy::missing_safety_doc)]

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

struct HelloWorldModule;

impl bindings::exports::hermes::ipfs::event::Guest for HelloWorldModule {
    #[doc = r" Triggers when a message is received on a topic."]
    fn on_topic(_message: PubsubMessage) -> bool {
        true
    }
}

impl bindings::exports::hermes::cardano::event_on_block::Guest for HelloWorldModule {
    fn on_cardano_block(_blockchain: CardanoBlockchainId, _block: CardanoBlock, _source: BlockSrc) {
    }
}

impl bindings::exports::hermes::cardano::event_on_rollback::Guest for HelloWorldModule {
    fn on_cardano_rollback(_blockchain: CardanoBlockchainId, _slot: u64) {}
}

impl bindings::exports::hermes::cardano::event_on_txn::Guest for HelloWorldModule {
    fn on_cardano_txn(
        _blockchain: CardanoBlockchainId, _slot: u64, _txn_index: u32, _txn: CardanoTxn,
    ) {
    }
}

impl bindings::exports::hermes::cron::event::Guest for HelloWorldModule {
    fn on_cron(_event: CronTagged, _last: bool) -> bool {
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
    fn reply(
        _body: Bstr, _headers: Headers, _path: String, _method: String,
    ) -> Option<HttpResponse> {
        None
    }
}

impl bindings::exports::hermes::kv_store::event::Guest for HelloWorldModule {
    fn kv_update(_key: String, _value: bindings::exports::hermes::kv_store::event::KvValues) {}
}

impl bindings::exports::wasi::http::incoming_handler::Guest for HelloWorldModule {
    fn handle(_request: IncomingRequest, _response_out: ResponseOutparam) {}
}

impl bindings::exports::hermes::integration_test::event::Guest for HelloWorldModule {
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
