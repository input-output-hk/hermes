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

use bindings::{
    exports::hermes::http_gateway::event::{Headers, HttpGatewayResponse},
    hermes::{binary::api::Bstr, cron::api::CronTagged, ipfs::api::PubsubMessage},
    wasi::http::types::{IncomingRequest, ResponseOutparam},
};
struct FailedInitApp;

impl bindings::exports::hermes::ipfs::event::Guest for FailedInitApp {
    fn on_topic(_message: PubsubMessage) -> bool {
        true
    }
}

impl bindings::exports::hermes::cardano::event_on_block::Guest for FailedInitApp {
    fn on_cardano_block(
        _subscription_id: &bindings::exports::hermes::cardano::event_on_block::SubscriptionId,
        _block: &bindings::exports::hermes::cardano::event_on_block::Block,
    ) -> () {
    }
}

impl bindings::exports::hermes::cardano::event_on_immutable_roll_forward::Guest for FailedInitApp {
    fn on_cardano_immutable_roll_forward(
        _subscription_id: &bindings::exports::hermes::cardano::event_on_immutable_roll_forward::SubscriptionId,
        _block: &bindings::exports::hermes::cardano::event_on_immutable_roll_forward::Block,
    ) -> () {
    }
}

impl bindings::exports::hermes::cron::event::Guest for FailedInitApp {
    fn on_cron(
        _event: CronTagged,
        _last: bool,
    ) -> bool {
        false
    }
}

impl bindings::exports::hermes::init::event::Guest for FailedInitApp {
    fn init() -> bool {
        test_log("module explicitly failing to initialize");
        false
    }
}

impl bindings::exports::hermes::http_request::event::Guest for FailedInitApp {
    fn on_http_response(
        _request_id: Option<u64>,
        _response: Vec<u8>,
    ) {
    }
}

impl bindings::exports::hermes::http_gateway::event::Guest for FailedInitApp {
    fn reply(
        _body: Bstr,
        _headers: Headers,
        _path: String,
        _method: String,
    ) -> Option<HttpGatewayResponse> {
        None
    }
}

impl bindings::exports::hermes::kv_store::event::Guest for FailedInitApp {
    fn kv_update(
        _key: String,
        _value: bindings::exports::hermes::kv_store::event::KvValues,
    ) {
    }
}

impl bindings::exports::wasi::http::incoming_handler::Guest for FailedInitApp {
    fn handle(
        _request: IncomingRequest,
        _response_out: ResponseOutparam,
    ) {
    }
}

impl bindings::exports::hermes::integration_test::event::Guest for FailedInitApp {
    fn test(
        _test: u32,
        _run: bool,
    ) -> Option<bindings::exports::hermes::integration_test::event::TestResult> {
        None
    }

    fn bench(
        _test: u32,
        _run: bool,
    ) -> Option<bindings::exports::hermes::integration_test::event::TestResult> {
        None
    }
}

bindings::export!(FailedInitApp with_types_in bindings);

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
