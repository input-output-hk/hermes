use crate::{hermes::*, HttpProxyComponent};

impl exports::hermes::integration_test::event::Guest for HttpProxyComponent {
    fn test(
        _test: u32, _run: bool,
    ) -> Option<exports::hermes::integration_test::event::TestResult> {
        None
    }

    fn bench(
        _test: u32, _run: bool,
    ) -> Option<exports::hermes::integration_test::event::TestResult> {
        None
    }
}

impl exports::hermes::cardano::event_on_immutable_roll_forward::Guest for HttpProxyComponent {
    fn on_cardano_immutable_roll_forward(
        _subscription_id: &hermes::cardano::event_on_immutable_roll_forward::SubscriptionId,
        _block: &hermes::cardano::event_on_immutable_roll_forward::Block,
    ) {
    }
}

impl exports::hermes::cardano::event_on_block::Guest for HttpProxyComponent {
    fn on_cardano_block(
        _subscription_id: &hermes::cardano::event_on_block::SubscriptionId,
        _block: &hermes::cardano::event_on_block::Block,
    ) {
    }
}

impl exports::hermes::cron::event::Guest for HttpProxyComponent {
    fn on_cron(_event: hermes::cron::api::CronTagged, _last: bool) -> bool {
        false
    }
}

impl exports::hermes::ipfs::event::Guest for HttpProxyComponent {
    fn on_topic(_message: hermes::ipfs::api::PubsubMessage) -> bool {
        false
    }
}

impl exports::hermes::kv_store::event::Guest for HttpProxyComponent {
    fn kv_update(_key: String, _value: hermes::kv_store::api::KvValues) {}
}

impl exports::wasi::http::incoming_handler::Guest for HttpProxyComponent {
    fn handle(
        _request: wasi::http::types::IncomingRequest,
        _response_out: wasi::http::types::ResponseOutparam,
    ) {
    }
}

impl exports::hermes::http_request::event::Guest for HttpProxyComponent {
    fn on_http_response(_request_id: Option<u64>, _response: Vec<u8>) -> () {}
}

fn log_cardano_age(days: f64) {
    const FILE: &str = "cardano_age/src/lib.rs";

    let msg = format!("Cardano is live for {days} days!");

    crate::hermes::hermes::logging::api::log(
        crate::hermes::hermes::logging::api::Level::Info,
        Some(&FILE),
        None,
        None,
        None,
        None,
        &msg,
        None,
    );
}

impl exports::hermes::init::event::Guest for HttpProxyComponent {
    fn init() -> bool {
        const CARDANO_LAUNCH_SECONDS: u64 = 1506246291;
        const SECONDS_IN_A_DAY: u64 = 24 * 60 * 60;


        let elapsed_seconds = crate::hermes::wasi::clocks::wall_clock::now()
            .seconds
            .saturating_sub(CARDANO_LAUNCH_SECONDS);

        let elapsed_days = elapsed_seconds as f64 / SECONDS_IN_A_DAY as f64;
        log_cardano_age(elapsed_days);

        true
    }
}