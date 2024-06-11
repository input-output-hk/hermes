//! Hermes SQLite module integration test with WASM runtime.
//! Generate `hermes.rs` with `earthly +gen-bindings` before writing the test.

// Allow everything since this is generated code.
#[allow(clippy::all, unused)]
mod hermes;

use hermes::{
    exports::hermes::integration_test::event::TestResult,
    hermes::{
        sqlite,
        cardano::api::{BlockSrc, CardanoBlock, CardanoBlockchainId, CardanoTxn},
        cron::api::CronTagged,
        kv_store::api::KvValues,
    },
    wasi::http::types::{IncomingRequest, ResponseOutparam},
};

struct TestItem {
    name: &'static str,
    executor: fn() -> bool,
}

const TESTS: &'static [TestItem] = &[
    TestItem {
        name: "open-database-persistent-simple",
        executor: || -> bool {
            let result = sqlite::api::open(false, false);

            result.is_ok()
        }
    },
    TestItem {
        name: "open-database-memory-simple",
        executor: || -> bool {
            let result = sqlite::api::open(false, true);

            result.is_ok()
        }
    }
];

const BENCHES: &'static [TestItem] = &[
    TestItem {
        name: "bench-simple",
        executor: || -> bool {
            false
        }
    }
];

struct TestComponent;

impl hermes::exports::hermes::integration_test::event::Guest for TestComponent {
    fn test(test: u32, run: bool) -> Option<TestResult> {
        TESTS.get(test as usize).map(|item| {
            TestResult {
                name: String::from(item.name),
                status: {
                    if run {
                        (item.executor)()
                    } else {
                        true
                    }
                }
            }
        })
    }

    fn bench(test: u32, run: bool) -> Option<TestResult> {
        BENCHES.get(test as usize).map(|item| {
            TestResult {
                name: String::from(item.name),
                status: {
                    if run {
                        (item.executor)()
                    } else {
                        true
                    }
                }
            }
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

impl hermes::exports::hermes::kv_store::event::Guest for TestComponent {
    fn kv_update(_key: String, _value: KvValues) {}
}

impl hermes::exports::wasi::http::incoming_handler::Guest for TestComponent {
    fn handle(_request: IncomingRequest, _response_out: ResponseOutparam) {}
}

hermes::export!(TestComponent with_types_in hermes);
