// Allow everything since this is generated code.
#[allow(clippy::all, unused)]
mod hermes;
mod stub;

struct TestComponent;

impl hermes::exports::hermes::cardano::event_on_block::Guest for TestComponent {
    fn on_cardano_block(
        subscription_id: &hermes::exports::hermes::cardano::event_on_block::SubscriptionId,
        block: &hermes::exports::hermes::cardano::event_on_block::Block,
    ) {
        let mut txn_hash = None;
        let slot = block.get_slot();
        let is_immutable = block.is_immutable();
        let is_rollback = block.is_rollback();
        let network = subscription_id.get_network();
        if let Ok(txn) = block.get_txn(0) {
            txn_hash = txn.get_txn_hash();
        }

        hermes::hermes::logging::api::log(
            hermes::hermes::logging::api::Level::Warn,
            None,
            None,
            None,
            None,
            None,
            format!("✈️ - on_cardano_block event trigger - subscription ID: {subscription_id:?}, network: {network:?}, slot: {slot:?}, is rollback: {is_rollback:?}, is immutable: {is_immutable}, txn hash: {txn_hash:?}").as_str(),
            None,
        );
    }
}

impl hermes::exports::hermes::cardano::event_on_immutable_roll_forward::Guest for TestComponent {
    fn on_cardano_immutable_roll_forward(
        subscription_id: &hermes::exports::hermes::cardano::event_on_immutable_roll_forward::SubscriptionId,
        block: &hermes::exports::hermes::cardano::event_on_immutable_roll_forward::Block,
    ) {
        let slot = block.get_slot();
        let network = subscription_id.get_network();
        hermes::hermes::logging::api::log(
            hermes::hermes::logging::api::Level::Trace,
            None,
            None,
            None,
            None,
            None,
            format!("🚄 - on_cardano_immutable_roll_forward event trigger - subscription ID: {subscription_id:?}, network: {network:?}, slot: {slot:?}").as_str(),
            None,
        );
    }
}

impl hermes::exports::hermes::init::event::Guest for TestComponent {
    fn init() -> bool {
        hermes::hermes::logging::api::log(
            hermes::hermes::logging::api::Level::Trace,
            None,
            None,
            None,
            None,
            None,
            format!("🍊 Init event trigger").as_str(),
            None,
        );

        let subscribe_from = hermes::hermes::cardano::api::SyncSlot::Tip;
        let network = hermes::hermes::cardano::api::CardanoNetwork::Preview;

        let network_resource = hermes::hermes::cardano::api::Network::new(network).unwrap();
        let subscription_id_resource = network_resource.subscribe_block(subscribe_from).unwrap();
        hermes::hermes::logging::api::log(
            hermes::hermes::logging::api::Level::Trace,
            None,
            None,
            None,
            None,
            None,
            format!("🎧 Network {network:?}, Subscribe to a block from {subscribe_from:?}, with subscription id: {subscription_id_resource:?}").as_str(),
            None,
        );

        true
    }
}

hermes::export!(TestComponent with_types_in hermes);
