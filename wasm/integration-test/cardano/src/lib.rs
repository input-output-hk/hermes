#![allow(clippy::all, unused)]
mod hermes;

use hermes::{
    exports::hermes::{
        http_gateway::event::{Bstr, Headers, HttpGatewayResponse},
        integration_test::event::TestResult,
    },
    hermes::{
        cardano
        cron::api::CronTagged,
        kv_store::api::KvValues,
        ipfs::api::PubsubMessage,
    },
    wasi::http::types::{IncomingRequest, ResponseOutparam},
};

use pallas_traverse::MultiEraBlock;

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
    let Some(tx_resource) = block_resource.get_txn(index as u16) else {
        return false;
    };
    let hash = if let Some(tx) = tx_resource.get_txn_hash() {
        tuple_to_array(tx).to_vec()
    } else {
        return false;
    };

    let encode_hash = hex::encode(hash);

    let tx_check = tx_resource.get_metadata().is_some() && encode_hash == tx_hash;
    let network_check = network_resource.get_tips().is_some();
    let block_check = block_resource.is_immutable() == true
        && block_resource.get_slot() == slot
        && block_resource.is_rollback() == false;

    network_check && block_check && tx_check
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
    match id.get_network() {
        cardano::api::CardanoNetwork::Preprod => true,
        _ => false,
    }
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
        path: String,
        method: String,
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
    fn on_http_response(_request_id: Option<u64>, _response: Vec::<u8>) -> () {}
}

hermes::export!(TestComponent with_types_in hermes);