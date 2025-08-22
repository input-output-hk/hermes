use crate::{
    bindings::{
        self,
        exports::hermes::http_gateway::event::{Headers, HttpGatewayResponse},
        hermes::{binary::api::Bstr, cron::api::CronTagged, ipfs::api::PubsubMessage},
        wasi::http::types::{IncomingRequest, ResponseOutparam},
    },
    HttpRequestApp,
};

impl bindings::exports::hermes::ipfs::event::Guest for HttpRequestApp {
    fn on_topic(_message: PubsubMessage) -> bool {
        true
    }
}

impl bindings::exports::hermes::cardano::event_on_block::Guest for HttpRequestApp {
    fn on_cardano_block(
        _subscription_id: &bindings::exports::hermes::cardano::event_on_block::SubscriptionId,
        _block: &bindings::exports::hermes::cardano::event_on_block::Block,
    ) {
    }
}

impl bindings::exports::hermes::cardano::event_on_immutable_roll_forward::Guest for HttpRequestApp {
    fn on_cardano_immutable_roll_forward(
        _subscription_id: &bindings::exports::hermes::cardano::event_on_immutable_roll_forward::SubscriptionId,
        _block: &bindings::exports::hermes::cardano::event_on_immutable_roll_forward::Block,
    ) {
    }
}

impl bindings::exports::hermes::cron::event::Guest for HttpRequestApp {
    fn on_cron(
        _event: CronTagged,
        _last: bool,
    ) -> bool {
        false
    }
}

impl bindings::exports::hermes::http_gateway::event::Guest for HttpRequestApp {
    fn reply(
        _body: Bstr,
        _headers: Headers,
        _path: String,
        _method: String,
    ) -> Option<HttpGatewayResponse> {
        None
    }
}

impl bindings::exports::hermes::kv_store::event::Guest for HttpRequestApp {
    fn kv_update(
        _key: String,
        _value: bindings::exports::hermes::kv_store::event::KvValues,
    ) {
    }
}

impl bindings::exports::wasi::http::incoming_handler::Guest for HttpRequestApp {
    fn handle(
        _request: IncomingRequest,
        _response_out: ResponseOutparam,
    ) {
    }
}

impl bindings::exports::hermes::integration_test::event::Guest for HttpRequestApp {
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
