use super::{hermes::*, TestComponent};

impl exports::hermes::integration_test::event::Guest for TestComponent {
    fn test(
        _test: u32,
        _run: bool,
    ) -> Option<exports::hermes::integration_test::event::TestResult> {
        None
    }

    fn bench(
        _test: u32,
        _run: bool,
    ) -> Option<exports::hermes::integration_test::event::TestResult> {
        None
    }
}

impl exports::hermes::cardano::event_on_block::Guest for TestComponent {
    fn on_cardano_block(
        _blockchain: hermes::cardano::api::CardanoBlockchainId,
        _block: hermes::cardano::api::CardanoBlock,
        _source: hermes::cardano::api::BlockSrc,
    ) {
    }
}

impl exports::hermes::cardano::event_on_rollback::Guest for TestComponent {
    fn on_cardano_rollback(_blockchain: hermes::cardano::api::CardanoBlockchainId, _slot: u64) {}
}

impl exports::hermes::cardano::event_on_txn::Guest for TestComponent {
    fn on_cardano_txn(
        _blockchain: hermes::cardano::api::CardanoBlockchainId,
        _slot: u64,
        _txn_index: u32,
        _txn: hermes::cardano::api::CardanoTxn,
    ) {
    }
}

impl exports::hermes::cron::event::Guest for TestComponent {
    fn on_cron(_event: hermes::cron::api::CronTagged, _last: bool) -> bool {
        false
    }
}

impl exports::hermes::ipfs::event::Guest for TestComponent {
    fn on_topic(_message: hermes::ipfs::api::PubsubMessage) -> bool {
        false
    }
}

impl exports::hermes::kv_store::event::Guest for TestComponent {
    fn kv_update(_key: String, _value: hermes::kv_store::api::KvValues) {}
}

impl exports::hermes::http_gateway::event::Guest for TestComponent {
    fn reply(
        _body: exports::hermes::http_gateway::event::Bstr,
        _headers: exports::hermes::http_gateway::event::Headers,
        _path: String,
        _method: String,
    ) -> Option<exports::hermes::http_gateway::event::HttpGatewayResponse> {
        None
    }
}

impl exports::wasi::http::incoming_handler::Guest for TestComponent {
    fn handle(
        _request: wasi::http::types::IncomingRequest,
        _response_out: wasi::http::types::ResponseOutparam,
    ) {
    }
}

impl exports::hermes::http_request::event::Guest for TestComponent {
    fn on_http_response(_request_id: Option<u64>, _response: Vec<u8>) -> () {}
}