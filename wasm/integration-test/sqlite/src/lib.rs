// Allow everything since this is generated code.
#[allow(clippy::all, unused)]
mod hermes;

use hermes::{
    exports::hermes::integration_test::event::TestResult,
    hermes::{
        cardano::{
            self,
            api::{BlockSrc, CardanoBlock, CardanoBlockchainId, CardanoTxn, Slot},
        },
        cron::api::CronTagged,
        kv_store::api::KvValues,
    },
    wasi::http::types::{IncomingRequest, ResponseOutparam},
};

struct TestComponent;

impl hermes::exports::hermes::integration_test::event::Guest for TestComponent {
    fn test(test: u32, run: bool) -> Option<TestResult> {
        todo!()
    }

    fn bench(_test: u32, _run: bool) -> Option<TestResult> {
        None
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
