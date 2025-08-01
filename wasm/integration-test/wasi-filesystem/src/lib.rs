use hermes::exports::hermes::http_gateway::event::{Bstr, Headers, HttpGatewayResponse};
use hermes::exports::hermes::integration_test::event::TestResult;

mod hermes;
mod tests;

struct TestComponent;

impl hermes::exports::hermes::init::event::Guest for TestComponent {
    fn init() -> bool {
        true
    }
}

impl hermes::exports::hermes::cron::event::Guest for TestComponent {
    fn on_cron(_event: hermes::exports::hermes::cron::event::CronTagged, _last: bool) -> bool {
        true
    }
}

impl hermes::exports::hermes::ipfs::event::Guest for TestComponent {
    fn on_topic(_message: hermes::exports::hermes::ipfs::event::PubsubMessage) -> bool {
        true
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

impl hermes::exports::hermes::kv_store::event::Guest for TestComponent {
    fn kv_update(_key: String, _value: hermes::exports::hermes::kv_store::event::KvValues) {}
}

impl hermes::exports::hermes::integration_test::event::Guest for TestComponent {
    fn test(test: u32, run: bool) -> Option<TestResult> {
        let test_fns = tests::test_fns();

        if let Some((test_name, test_fn)) = test_fns.get(test as usize) {
            let status = if run {
                test_fn()
                    .map_err(|e| {
                        eprintln!("{e:?}");
                        e
                    })
                    .is_ok()
            } else {
                true
            };

            Some(TestResult {
                name: test_name.to_string(),
                status,
            })
        } else {
            None
        }
    }

    fn bench(_test: u32, _run: bool) -> Option<TestResult> {
        None
    }
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
    fn handle(
        _request: hermes::exports::wasi::http::incoming_handler::IncomingRequest,
        _response_out: hermes::exports::wasi::http::incoming_handler::ResponseOutparam,
    ) {
    }
}

impl hermes::exports::hermes::http_request::event::Guest for TestComponent {
    fn on_http_response(_request_id: Option<u64>, _response: Vec<u8>) -> () {}
}

hermes::export!(TestComponent with_types_in hermes);
