#[allow(clippy::all, unused)]
mod hermes;
use hermes::{
    exports::hermes::{
        http_gateway::event::{Bstr, Headers, HttpGatewayResponse},
        integration_test::event::TestResult,
    },
    hermes::{
        cardano::api::{BlockSrc, CardanoBlock, CardanoBlockchainId, CardanoTxn},
        cron::api::CronTagged,
        ipfs::api::{self as ipfs_api, IpfsContent, PeerId, PubsubMessage},
        kv_store::api::KvValues,
    },
    wasi::http::types::{IncomingRequest, ResponseOutparam},
};

const IPFS_DEMO_FILE: &[u8] = b"ipfs file uploaded from wasm";
struct TestComponent;

fn test_file_add_and_get_and_pin(run: bool) -> Option<TestResult> {
    let status = if run {
        if let Ok(ipfs_path) = ipfs_api::file_add(&IPFS_DEMO_FILE.to_vec()) {
            let contents_match = ipfs_api::file_get(&ipfs_path)
                .map_or(false, |ipfs_file| ipfs_file == IPFS_DEMO_FILE);
            let expected_status_is_true =
                ipfs_api::file_pin(&ipfs_path).map_or(false, |status| status);
            contents_match && expected_status_is_true
        } else {
            false
        }
    } else {
        true
    };

    Some(TestResult {
        name: "IPFS File Add/Get".to_string(),
        status,
    })
}
fn test_dht_put_and_get(run: bool) -> Option<TestResult> {
    let key = b"my-key".to_vec();
    let value = b"demo dht value".to_vec();
    let status = if run {
        if let Ok(dht_value) = ipfs_api::dht_put(&key, &value) {
            if let Ok(dht_value) = ipfs_api::dht_get(&key) {
                dht_value == value
            } else {
                false
            }
        } else {
            false
        }
    } else {
        true
    };
    Some(TestResult {
        name: "IPFS DHT Put/Get".to_string(),
        status,
    })
}

fn test_pubsub_topic_subscribe(run: bool) -> Option<TestResult> {
    let topic = "demo-topic".to_string();
    let status = if run {
        ipfs_api::pubsub_subscribe(&topic).is_ok()
    } else {
        true
    };
    Some(TestResult {
        name: "IPFS Pubsub Subscribe To Topic".to_string(),
        status,
    })
}

fn test_pubsub_topic_publish(run: bool) -> Option<TestResult> {
    let topic = "demo-topic".to_string();
    let message = b"demo message".to_vec();
    let status = if run {
        ipfs_api::pubsub_publish(&topic, &message).is_ok()
    } else {
        false
    };
    Some(TestResult {
        name: "IPFS Pubsub Publish To Topic".to_string(),
        status,
    })
}

fn test_peer_evict(run: bool) -> Option<TestResult> {
    let peer = "12D3KooWMisUYkyVLdVsMcJukAVhPHjGKaJNZG8BKNwh5WGnGk8P".to_string();
    let status = if run {
        if let Ok(status) = ipfs_api::peer_evict(&peer) {
            status
        } else {
            false
        }
    } else {
        false
    };
    Some(TestResult {
        name: "IPFS Ban Peer".to_string(),
        status,
    })
}

fn test_validate_dht_value(run: bool) -> Option<TestResult> {
    let key = b"valid-value".to_vec();
    let value = b"demo dht value".to_vec();
    let content = IpfsContent::Dht((key, value));
    let status_a = if run {
        if let Ok(is_valid) = ipfs_api::ipfs_content_validate(&content) {
            is_valid
        } else {
            false
        }
    } else {
        false
    };
    let key = b"invalid-value".to_vec();
    let value = b"".to_vec();
    let content = IpfsContent::Dht((key, value));
    let status_b = if run {
        if let Ok(is_valid) = ipfs_api::ipfs_content_validate(&content) {
            !is_valid
        } else {
            false
        }
    } else {
        false
    };
    let status = status_a && status_b;
    Some(TestResult {
        name: "IPFS Validate DHT Value".to_string(),
        status,
    })
}
impl hermes::exports::hermes::integration_test::event::Guest for TestComponent {
    fn test(test: u32, run: bool) -> Option<TestResult> {
        match test {
            0 => {
                //
                test_file_add_and_get_and_pin(run)
            }
            1 => {
                //
                test_dht_put_and_get(run)
            }
            2 => {
                // Test IPFS Pubsub
                test_pubsub_topic_subscribe(run)
            }
            3 => {
                // Test IPFS Pubsub
                test_pubsub_topic_publish(run)
            }
            4 => {
                // Test IPFS Peer Evict
                test_peer_evict(run)
            }
            5 => {
                // Test IPFS Validate DHT Value
                test_validate_dht_value(run)
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
    ) -> Option<HttpGatewayResponse> {
        None
    }
}

impl hermes::exports::wasi::http::incoming_handler::Guest for TestComponent {
    fn handle(_request: IncomingRequest, _response_out: ResponseOutparam) {}
}

impl hermes::exports::hermes::http_request::event::Guest for TestComponent {
    fn on_http_response(_request_id: Option<u64>, _response: Vec<u8>) -> () {}
}

hermes::export!(TestComponent with_types_in hermes);
