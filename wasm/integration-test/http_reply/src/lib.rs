//! Hermes http reply module integration test with WASM runtime.
//! Generate `hermes.rs` with `earthly +gen-bindings` before writing the test.

// Allow everything since this is generated code.
#![allow(clippy::all, unused)]
mod hermes;
use crate::hermes::exports::hermes::http_gateway::event::Guest;
use hermes::{
    exports::hermes::{
        http_gateway::event::{Bstr, Headers, HttpGatewayResponse},
        integration_test::event::TestResult,
    },
    hermes::{
        cron::api::CronTagged,
        ipfs::api::{self as ipfs_api, IpfsContent, PeerId, PubsubMessage},
        kv_store::api::KvValues,
    },
    wasi::http::types::{IncomingRequest, ResponseOutparam},
};

struct TestComponent;

impl hermes::exports::hermes::integration_test::event::Guest for TestComponent {
    fn test(test: u32, run: bool) -> Option<TestResult> {
        match test {
            0 => test_http_reply(run),

            _ => None,
        }
    }

    fn bench(_test: u32, _run: bool) -> Option<TestResult> {
        None
    }
}

fn test_http_reply(run: bool) -> Option<TestResult> {
    let body_bytes: Vec<u8> = (0..1024).map(|_| 0 as u8).collect();
    let header = vec![("key".to_string(), vec!["values".to_string()])];
    let reply = TestComponent::reply(body_bytes, header, "path".to_string(), "method".to_string());

    let status = if let Some(reply) = reply {
        match reply {
            HttpGatewayResponse::Http(http_resp) => http_resp.code == 200,
            HttpGatewayResponse::InternalRedirect(_) => true, // or false, depending on your test logic
        }
    } else {
        false
    };

    Some(TestResult {
        name: "HTTP reply".to_string(),
        status,
    })
}

impl hermes::exports::hermes::cardano::event_on_immutable_roll_forward::Guest for TestComponent {
    fn on_cardano_immutable_roll_forward(
        _subscription_id: &hermes::exports::hermes::cardano::event_on_immutable_roll_forward::SubscriptionId,
        _block: &hermes::exports::hermes::cardano::event_on_immutable_roll_forward::Block,
    ) {
    }
}

impl hermes::exports::hermes::cardano::event_on_block::Guest for TestComponent {
    fn on_cardano_block(
        _subscription_id: &hermes::exports::hermes::cardano::event_on_block::SubscriptionId,
        _block: &hermes::exports::hermes::cardano::event_on_block::Block,
    ) {
    }
}

impl hermes::exports::hermes::cron::event::Guest for TestComponent {
    fn on_cron(_event: CronTagged, _last: bool) -> bool {
        false
    }
}

impl hermes::exports::hermes::init::event::Guest for TestComponent {
    fn init() -> bool {
        true
    }
}

impl hermes::exports::hermes::ipfs::event::Guest for TestComponent {
    fn on_topic(_message: PubsubMessage) -> bool {
        false
    }
}

impl hermes::exports::hermes::kv_store::event::Guest for TestComponent {
    fn kv_update(_key: String, _value: KvValues) {}
}

impl hermes::exports::hermes::http_gateway::event::Guest for TestComponent {
    fn reply(
        _body: Bstr,
        _headers: Headers,
        _path: String,
        _method: String,
    ) -> Option<HttpGatewayResponse> {
        None
    }
}

impl hermes::exports::wasi::http::incoming_handler::Guest for TestComponent {
    fn handle(_request: IncomingRequest, _response_out: ResponseOutparam) {}
}

impl hermes::exports::hermes::http_request::event::Guest for TestComponent {
    fn on_http_response(_request_id: Option<u64>, _response: Vec<u8>) -> () {}
}

hermes::export!(TestComponent with_types_in hermes);
