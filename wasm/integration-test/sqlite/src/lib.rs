//! Hermes SQLite module integration test with WASM runtime.
//! Generate `hermes.rs` with `earthly +gen-bindings` before writing the test.

// Allow everything since this is generated code.
#[allow(clippy::all, unused)]
mod hermes;
mod test;

use hermes::{
    exports::hermes::{
        integration_test::event::TestResult,
        http_gateway::event::{Bstr, Headers, HttpResponse}
    },
    hermes::{
        cardano::api::{BlockSrc, CardanoBlock, CardanoBlockchainId, CardanoTxn},
        cron::api::CronTagged,
        ipfs::api::PubsubMessage,
        kv_store::api::KvValues,
        sqlite,
    },
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

impl hermes::exports::hermes::cardano::event_on_block::Guest for TestComponent {
    fn on_cardano_block(_blockchain: CardanoBlockchainId, _block: CardanoBlock, _source: BlockSrc) {
    }
}

impl hermes::exports::hermes::cardano::event_on_rollback::Guest for TestComponent {
    fn on_cardano_rollback(_blockchain: CardanoBlockchainId, _slot: u64) {}
}

impl hermes::exports::hermes::cardano::event_on_txn::Guest for TestComponent {
    fn on_cardano_txn(
        _blockchain: CardanoBlockchainId,
        _slot: u64,
        _txn_index: u32,
        _txn: CardanoTxn,
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

impl hermes::exports::hermes::http_gateway::event::Guest for TestComponent {
    fn reply(
        _body: hermes::exports::hermes::http_gateway::event::Bstr,
        _headers: hermes::exports::hermes::http_gateway::event::Headers,
        _path: String,
        _method: String,
    ) -> Option<hermes::exports::hermes::http_gateway::event::HttpResponse> {
        None
    }
}

impl hermes::exports::hermes::kv_store::event::Guest for TestComponent {
    fn kv_update(_key: String, _value: KvValues) {}
}

impl hermes::exports::hermes::http_gateway::event::Guest for TestComponent {
    fn reply(_body: Bstr, _headers: Headers, _path: String, method: String,) -> Option<HttpResponse> {
        None
    }
}

impl hermes::exports::wasi::http::incoming_handler::Guest for TestComponent {
    fn handle(_request: IncomingRequest, _response_out: ResponseOutparam) {}
}

hermes::export!(TestComponent with_types_in hermes);
