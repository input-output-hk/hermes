wit_bindgen::generate!({
    world: "hermes:app/hermes",
    path: "../../wasi/wit",
    inline: "
        package hermes:app;

        world hermes {
            import hermes:ipfs/api;
            
            export hermes:integration-test/event;
        }
    ",
    generate_all,
});

export!(TestComponent);

use hermes::integration_test::api::TestResult;
use hermes::ipfs::api::{self as ipfs_api, IpfsContent};

const IPFS_DEMO_FILE: &[u8] = b"ipfs file uploaded from wasm";
struct TestComponent;

fn test_file_add_and_get_and_pin(run: bool) -> Option<TestResult> {
    let status = if run {
        match ipfs_api::file_add(&IPFS_DEMO_FILE.to_vec()) {
            Ok(ipfs_api::FileAddResult {
                file_path: ipfs_path,
                ..
            }) => {
                let contents_match = ipfs_api::file_get(&ipfs_path)
                    .map_or(false, |ipfs_file| ipfs_file == IPFS_DEMO_FILE);
                let expected_status_is_true =
                    ipfs_api::file_pin(&ipfs_path).map_or(false, |status| status);
                contents_match && expected_status_is_true
            }
            _ => false,
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
impl exports::hermes::integration_test::event::Guest for TestComponent {
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
