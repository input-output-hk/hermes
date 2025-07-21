#![allow(unused)]

mod bindings {
    #![allow(clippy::missing_safety_doc)]

    wit_bindgen::generate!({
        world: "hermes",
        path: "/home/magister/IOHK/hermes/wasm/wasi/wit",
        generate_all,
    });
}

use std::error::Error;
use std::thread::{self, sleep};

use bindings::hermes::ipfs::api::PubsubMessage;
use bindings::wasi::clocks;
use bindings::wasi::http::types::{IncomingRequest, ResponseOutparam};
use bindings::{
    exports::hermes::http_gateway::event::{Headers, HttpResponse},
    hermes::{
        cardano::api::{BlockSrc, CardanoBlock, CardanoBlockchainId, CardanoTxn},
        cron::api::CronTagged,
    },
};

use bindings::hermes::binary::api::Bstr;
use bindings::wasi::random::random::get_random_u64;

use serde::{Deserialize, Serialize};

use crate::bindings::hermes::http_request::api::Payload;
use crate::bindings::wasi::clocks::wall_clock;

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

struct Response {
    pub body: Vec<u8>,
    pub request_id: Option<String>,
}

impl bindings::exports::hermes::init::event::Guest for HelloWorldModule {
    fn init() -> bool {
        let seconds = wall_clock::now().seconds;
        fastrand::seed(seconds);

        let random_numbers = std::iter::repeat_with(|| fastrand::u8(1..=10))
            .take(5)
            .collect::<Vec<_>>();
        bindings::hermes::logging::api::log(
            bindings::hermes::logging::api::Level::Trace,
            None,
            None,
            None,
            None,
            None,
            format!(
                "XXXX Athena is sorting {} numbers: {random_numbers:?}",
                random_numbers.len()
            )
            .as_str(),
            None,
        );

        random_numbers.iter().for_each(|x| {
            let payload = delayed_payload(*x);

            let send_result = bindings::hermes::http_request::api::send(&payload);
            bindings::hermes::logging::api::log(
                bindings::hermes::logging::api::Level::Trace,
                None,
                None,
                None,
                None,
                None,
                format!("XXXX send result: {send_result:?}, awaiting response").as_str(),
                None,
            );
        });

        true
    }
}

fn delayed_payload(secs: u8) -> Payload {
    let request_body = delayed_body(secs);

    let payload = Payload {
        host_uri: "https://httpbin.org".to_string(),
        port: 443,
        body: request_body.to_vec(),
        request_id: Some(secs as u64),
    };
    payload
}

fn delayed_body(secs: u8) -> Vec<u8> {
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
                "XXXX got http response for {request_id:?}: {}",
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
        test: u32,
        run: bool,
    ) -> Option<bindings::exports::hermes::integration_test::event::TestResult> {
        None
    }

    fn bench(
        test: u32,
        run: bool,
    ) -> Option<bindings::exports::hermes::integration_test::event::TestResult> {
        None
    }
}

bindings::export!(HelloWorldModule with_types_in bindings);
