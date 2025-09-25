wit_bindgen::generate!({
    world: "hermes:app/hermes",
    path: "../../wasi/wit",
    inline: "
        package hermes:app;

        world hermes {
            import hermes:cardano/api;
            
            export hermes:integration-test/event;
        }
    ",
    generate_all,
});

export!(TestComponent);

use hermes::integration_test::api::TestResult;
use hermes::cardano;

struct TestComponent;

fn test_get_data() -> bool {
    let slot = 91210205;
    let index = 0;
    let tx_hash = "ef831f1d3ce6134628e39d288876408fac5f81840b9d74b9ba07f36e56090bb1";

    let Ok(network_resource) = cardano::api::Network::new(cardano::api::CardanoNetwork::Preprod)
    else {
        return false;
    };
    let Some(block_resource) = network_resource.get_block(Some(slot), index) else {
        return false;
    };
    let Ok(tx_resource) = block_resource.get_txn(index as u16) else {
        return false;
    };
    let hash = if let Some(tx) = tx_resource.get_txn_hash() {
        tuple_to_array(tx).to_vec()
    } else {
        return false;
    };

    let encode_hash = hex::encode(hash);

    let network_check = network_resource.get_tips().is_some();
    let block_check = block_resource.is_immutable() == true
        && block_resource.get_slot() == slot
        && block_resource.is_rollback() == Ok(false);

    network_check && block_check
}

fn tuple_to_array(t: (u64, u64, u64, u64)) -> [u8; 32] {
    let mut array = [0u8; 32];
    array[0..8].copy_from_slice(&t.0.to_le_bytes());
    array[8..16].copy_from_slice(&t.1.to_le_bytes());
    array[16..24].copy_from_slice(&t.2.to_le_bytes());
    array[24..32].copy_from_slice(&t.3.to_le_bytes());
    array
}

fn test_subscribe_block() -> bool {
    let Ok(network_resource) = cardano::api::Network::new(cardano::api::CardanoNetwork::Preprod)
    else {
        return false;
    };
    let Ok(id) = network_resource.subscribe_block(cardano::api::SyncSlot::Genesis) else {
        return false;
    };
    true
}

impl exports::hermes::integration_test::event::Guest for TestComponent {
    fn test(test: u32, run: bool) -> Option<TestResult> {
        match test {
            0 => {
                let status = if run { test_get_data() } else { true };

                Some(TestResult {
                    name: "Get data".to_string(),
                    status,
                })
            }
            1 => {
                let status = if run { test_subscribe_block() } else { true };

                Some(TestResult {
                    name: "Subscribe block".to_string(),
                    status,
                })
            }
            _ => None,
        }
    }

    fn bench(test: u32, run: bool) -> Option<TestResult> {
        None
    }
}
