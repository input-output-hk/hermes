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

use crate::bindings::{hermes::http_request::api::Payload, wasi::clocks::wall_clock};

struct HelloWorldModule;

impl bindings::exports::hermes::ipfs::event::Guest for HelloWorldModule {
    #[doc = r" Triggers when a message is received on a topic."]
    fn on_topic(message: PubsubMessage) -> bool {
        println!("hello");
        true
    }
}

impl bindings::exports::hermes::cardano::event_on_block::Guest for HelloWorldModule {
    fn on_cardano_block(blockchain: CardanoBlockchainId, block: CardanoBlock, source: BlockSrc) {
        let v = bindings::hermes::cardano::api::get_txns(&block);

        bindings::hermes::logging::api::log(
            bindings::hermes::logging::api::Level::Trace,
            None,
            None,
            None,
            None,
            None,
            format!("FROM WASM MODULE: block tx count = {}", v.len()).as_str(),
            None,
        );
    }
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

impl bindings::exports::hermes::init::event::Guest for HelloWorldModule {
    fn init() -> bool {
        let settings_json = fs::read_to_string("/lib/test_module/settings.schema.json")
            .expect("cannot read settings file");
        let parsed_json: serde_json::Value =
            serde_json::from_str(&settings_json).expect("unable to parse settings as JSON");
        let http_server = parsed_json
            .get("http_server")
            .expect("missing http_server in settings");

        bindings::hermes::logging::api::log(
            bindings::hermes::logging::api::Level::Trace,
            None,
            None,
            None,
            None,
            None,
            format!("XXXXX - Sending HTTP request to {http_server}").as_str(),
            None,
        );

        let payload = payload();

        let send_result = bindings::hermes::http_request::api::send(&payload);
        bindings::hermes::logging::api::log(
            bindings::hermes::logging::api::Level::Trace,
            None,
            None,
            None,
            None,
            None,
            format!("XXXXX - Sending result: {send_result:?}, awaiting response").as_str(),
            None,
        );

        true
    }
}

fn payload() -> Payload {
    let request_body = delayed_body();

    let payload = Payload {
        host_uri: "https://httpbin.org".to_string(),
        port: 443,
        body: request_body.to_vec(),
        request_id: Some(42 as u64),
    };
    payload
}

fn delayed_body() -> Vec<u8> {
    let request_body = format!(
        "POST /post HTTP/1.1\r\n\
        Host: httpbin.org\r\n\
        Content-Type: application/json\r\n\
        Content-Length: 15\r\n\
        Connection: close\r\n\
        \r\n\
        {{\"key\":\"value\"}}"
    );
    // let request_body = format!(
    //     "POST /delay/{} HTTP/1.1\r\n\
    //     Host: httpbin.org\r\n\
    //     Content-Type: application/json\r\n\
    //     Content-Length: 15\r\n\
    //     \r\n\
    //     {{\"key\":\"value\"}}",
    //     secs
    // );
    request_body.into_bytes()
}

impl bindings::exports::hermes::http_request::event::Guest for HelloWorldModule {
    fn on_http_response(request_id: Option<u64>, response: Vec<u8>) {
        bindings::hermes::logging::api::log(
            bindings::hermes::logging::api::Level::Trace,
            None,
            None,
            None,
            None,
            None,
            format!(
                "XXXXX got http response for {request_id:?}: {}",
                String::from_utf8(response).unwrap(),
            )
            .as_str(),
            None,
        );
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
