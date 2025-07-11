#[allow(clippy::all, unused)]
mod hermes;

use pallas_traverse::tx;

use crate::hermes::{exports::hermes::cardano::event_on_block::SubscriptionId, hermes::cardano};

struct TestComponent;

fn test_get_data() -> bool {
    let slot = 91210205;
    let index = 0;
    let tx_hash = hex::encode("ef831f1d3ce6134628e39d288876408fac5f81840b9d74b9ba07f36e56090bb1")
        .expect("Invalid transaction hash");

    let Ok(network_resource) = cardano::api::Network::new(cardano::api::CardanoNetwork::Preprod)
    else {
        return false;
    };
    let Some(block_resource) = network_resource.get_block(Some(slot), 0) else {
        return false;
    };
    let Some(tx_resource) = block_resource.get_txn(0) else {
        return false;
    };
    let hash = if let Some(tx) = tx_resource.get_txn_hash() {
        tuple_to_array(tx).to_vec()
    } else {
        return false;
    };

    let tx_check = tx_resource.get_metadata().is_some() && hash == tx_hash;
    let network_check = network_resource.get_tips().is_some();
    let block_check = block_resource.is_immutable() == true
        && block_resource.get_slot() == slot
        && block_resource.is_rollback() == false;

    network_check && block_check && tx_check
}

fn tuple_to_array(t: (u64, u64, u64, u64)) -> [u64; 4] {
    [t.0, t.1, t.2, t.3]
}

fn test_subscribe_block() -> bool {
    let Ok(network_resource) = cardano::api::Network::new(cardano::api::CardanoNetwork::Preprod)
    else {
        return false;
    };
    let Ok(id) = network_resource.subscribe_block(cardano::api::SyncSlot::Genesis) else {
        return false;
    };
    id.get_network() == cardano::api::CardanoNetwork::Preprod
}

impl hermes::exports::hermes::integration_test::event::Guest for TestComponent {
    fn test(
        test: u32,
        run: bool,
    ) -> Option<hermes::exports::hermes::integration_test::event::TestResult> {
        match test {
            0 => {
                let status = if run { test_get_data() } else { true };

                Some(
                    hermes::exports::hermes::integration_test::event::TestResult {
                        name: "Get data".to_string(),
                        status,
                    },
                )
            }
            1 => {
                let status = if run { test_subscribe_block() } else { true };

                Some(
                    hermes::exports::hermes::integration_test::event::TestResult {
                        name: "Subscribe block".to_string(),
                        status,
                    },
                )
            }
            _ => None,
        }
    }

    fn bench(
        test: u32,
        run: bool,
    ) -> Option<hermes::exports::hermes::integration_test::event::TestResult> {
        None
    }
}

impl hermes::exports::hermes::cardano::event_on_immutable_roll_forward::Guest for TestComponent {
    fn on_cardano_immutable_roll_forward(
        subscription_id: hermes::exports::hermes::cardano::event_on_immutable_roll_forward::SubscriptionId,
        block: hermes::exports::hermes::cardano::event_on_immutable_roll_forward::Block,
    ) {
    }
}

impl hermes::exports::hermes::cardano::event_on_block::Guest for TestComponent {
    fn on_cardano_block(
        subscription_id: hermes::exports::hermes::cardano::event_on_block::SubscriptionId,
        block: hermes::exports::hermes::cardano::event_on_block::Block,
    ) {
    }
}

impl hermes::exports::hermes::cron::event::Guest for TestComponent {
    fn on_cron(event: hermes::exports::hermes::cron::event::CronTagged, last: bool) -> bool {
        false
    }
}

impl hermes::exports::hermes::http_gateway::event::Guest for TestComponent {
    fn reply(
        body: hermes::exports::hermes::http_gateway::event::Bstr,
        headers: hermes::exports::hermes::http_gateway::event::Headers,
        path: _rt::String,
        method: _rt::String,
    ) -> Option<hermes::exports::hermes::http_gateway::event::HttpResponse> {
        None
    }
}

impl hermes::exports::hermes::init::event::Guest for TestComponent {
    fn init() -> bool {
        true
    }
}

impl hermes::exports::hermes::ipfs::event::Guest for TestComponent {
    fn on_topic(message: hermes::exports::hermes::ipfs::event::PubsubMessage) -> bool {
        false
    }
}

impl hermes::exports::hermes::kv_store::event::Guest for TestComponent {
    fn kv_update(key: _rt::String, value: hermes::exports::hermes::kv_store::event::KvValues) {}
}

impl hermes::exports::wasi::http::incoming_handler::Guest for TestComponent {
    fn handle(
        request: hermes::exports::wasi::http::incoming_handler::IncomingRequest,
        response_out: hermes::exports::wasi::http::incoming_handler::ResponseOutparam,
    ) -> () {
    }
}

hermes::export!(TestComponent with_types_in hermes);
