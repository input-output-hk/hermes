use crate::{hermes, DbComponent};

impl hermes::exports::hermes::integration_test::event::Guest for DbComponent {
    fn test(
        _test: u32,
        _run: bool,
    ) -> Option<hermes::exports::hermes::integration_test::event::TestResult> {
        None
    }

    fn bench(
        _test: u32,
        _run: bool,
    ) -> Option<hermes::exports::hermes::integration_test::event::TestResult> {
        None
    }
}

impl hermes::exports::hermes::cardano::event_on_immutable_roll_forward::Guest for DbComponent {
    fn on_cardano_immutable_roll_forward(
        _subscription_id: &hermes::exports::hermes::cardano::event_on_immutable_roll_forward::SubscriptionId,
        _block: &hermes::exports::hermes::cardano::event_on_immutable_roll_forward::Block,
    ) {
    }
}

impl hermes::exports::hermes::cardano::event_on_block::Guest for DbComponent {
    fn on_cardano_block(
        _subscription_id: &hermes::exports::hermes::cardano::event_on_block::SubscriptionId,
        _block: &hermes::exports::hermes::cardano::event_on_block::Block,
    ) {
    }
}

impl hermes::exports::hermes::cron::event::Guest for DbComponent {
    fn on_cron(
        _event: hermes::hermes::cron::api::CronTagged,
        _last: bool,
    ) -> bool {
        false
    }
}

impl hermes::exports::hermes::ipfs::event::Guest for DbComponent {
    fn on_topic(_message: hermes::hermes::ipfs::api::PubsubMessage) -> bool {
        false
    }
}

impl hermes::exports::hermes::kv_store::event::Guest for DbComponent {
    fn kv_update(
        _key: String,
        _value: hermes::hermes::kv_store::api::KvValues,
    ) {
    }
}

impl hermes::exports::wasi::http::incoming_handler::Guest for DbComponent {
    fn handle(
        _request: hermes::wasi::http::types::IncomingRequest,
        _response_out: hermes::wasi::http::types::ResponseOutparam,
    ) {
    }
}

impl hermes::exports::hermes::http_request::event::Guest for DbComponent {
    fn on_http_response(
        _request_id: Option<u64>,
        _response: Vec<u8>,
    ) -> () {
    }
}

impl hermes::exports::hermes::init::event::Guest for DbComponent {
    fn init() -> bool {
        true
    }
}

impl hermes::exports::hermes::http_gateway::event::Guest for DbComponent {
    fn reply(
        _body: Vec<u8>,
        _headers: hermes::exports::hermes::http_gateway::event::Headers,
        path: String,
        method: String,
    ) -> Option<hermes::exports::hermes::http_gateway::event::HttpGatewayResponse> {
        None
    }
}
