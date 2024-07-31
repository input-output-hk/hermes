// Allow everything since this is generated code.
#[allow(clippy::all, unused)]
mod hermes;

use hermes::{
    exports::hermes::{
        http_gateway::event::{Bstr, Headers, HttpResponse},
        integration_test::event::TestResult,
    },
    hermes::{
        cardano::{
            self,
            api::{BlockSrc, CardanoBlock, CardanoBlockchainId, CardanoTxn, Slot},
        },
        cron::api::CronTagged,
        ipfs::api::PubsubMessage,
        kv_store::api::KvValues,
    },
    wasi::http::types::{IncomingRequest, ResponseOutparam},
};
use pallas_traverse::MultiEraBlock;

struct TestComponent;

fn test_fetch_block() -> bool {
    let block_slot = 56581108;
    let block_hash =
        hex::decode("e095cca24865b85812109a8a0af01a8a80c6a74d1f56cfe1830631423a806b46")
            .expect("valid block hash hex");

    let slot = Slot::Point((block_slot, block_hash.clone()));

    let Ok(block_cbor) = cardano::api::fetch_block(CardanoBlockchainId::Preprod, &slot) else {
        return false;
    };

    let Ok(block) = MultiEraBlock::decode(&block_cbor) else {
        return false;
    };

    block_slot == block.slot() && block_hash == block.hash().to_vec()
}

impl hermes::exports::hermes::integration_test::event::Guest for TestComponent {
    fn test(test: u32, run: bool) -> Option<TestResult> {
        match test {
            0 => {
                let status = if run { test_fetch_block() } else { true };

                Some(TestResult {
                    name: "Fetch block".to_string(),
                    status,
                })
            }

            _ => None,
        }
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
    ) -> Option<HttpResponse> {
        None
    }
}

impl hermes::exports::wasi::http::incoming_handler::Guest for TestComponent {
    fn handle(_request: IncomingRequest, _response_out: ResponseOutparam) {}
}

hermes::export!(TestComponent with_types_in hermes);
