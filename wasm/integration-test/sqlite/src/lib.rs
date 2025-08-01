//! Hermes SQLite module integration test with WASM runtime.
//! Generate `hermes.rs` with `earthly +gen-bindings` before writing the test.

// Allow everything since this is generated code.
#[allow(clippy::all, unused)]
mod hermes;
mod test;

use hermes::{
    exports::hermes::{
        http_gateway::event::{Bstr, Headers, HttpGatewayResponse},
        integration_test::event::TestResult,
    },
    hermes::{cron::api::CronTagged, ipfs::api::PubsubMessage, kv_store::api::KvValues, sqlite},
    wasi::http::types::{IncomingRequest, ResponseOutparam},
};
use test::*;

struct TestComponent;

impl hermes::exports::hermes::integration_test::event::Guest for TestComponent {
    fn test(test: u32, run: bool) -> Option<TestResult> {
        TESTS.get(test as usize).map(|item| TestResult {
            name: String::from(item.name),
            status: {
                if run {
                    (item.executor)().is_ok()
                } else {
                    true
                }
            },
        })
    }

    fn bench(test: u32, run: bool) -> Option<TestResult> {
        BENCHES.get(test as usize).map(|item| TestResult {
            name: String::from(item.name),
            status: {
                if run {
                    (item.executor)().is_ok()
                } else {
                    true
                }
            },
        })
    }
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
