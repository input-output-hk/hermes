//! Hermes RTE events inner implementations.

use shared::{
    bindings::hermes::cardano::{
        self,
        api::{Block, SubscriptionId},
    },
    utils::{
        log::{error, info, trace},
        sqlite,
    },
};

use crate::database::create_tables;

/// Initializes sqlite tables and cardano block subscription.
pub fn init() -> anyhow::Result<()> {
    info!(
        target: "staked_ada_indexer::init",
        "💫 Initializing Sqlite..."
    );

    let mut conn = sqlite::Connection::open(false)?;

    let mut _conn_volatile = sqlite::Connection::open(true)?;

    create_tables(&mut conn)?;

    info!(
        target: "staked_ada_indexer::init",
        "💫 Sqlite initialized. Setting up Cardano subscription..."
    );

    let subscribe_from = cardano::api::SyncSlot::Genesis;
    let network = cardano::api::CardanoNetwork::Preprod;

    let network_resource = cardano::api::Network::new(network)
        .inspect_err(|error| error!(error:%, network:?; "Failed to create network resource"))?;
    let subscription_id_resource = network_resource
        .subscribe_block(subscribe_from)
        .inspect_err(|error| error!(error:%, subscribe_from:?; "Failed to subscribe block from"))?;

    info!(
        target: "staked_ada_indexer::init",
        network:?,
        subscription_id_resource:?;
        "💫 Cardano subscription set up."
    );

    Ok(())
}

/// Records new transactions.
pub fn on_cardano_block(
    subscription_id: &SubscriptionId,
    block: &Block,
) -> anyhow::Result<()> {
    let _block = block.to_catalyst_type(subscription_id.get_network());
    let conn = sqlite::Connection::open(false)?;
    let _conn_volatile = sqlite::Connection::open(true)?;

    // Simple mock, propagating slot_no.
    let (slot_no,) = conn
        .prepare("SELECT ?")?
        .query_one_as::<(u64,)>(&[&block.get_slot().try_into()?])?;

    trace!(
        target: "staked_ada_indexer::on_cardano_block",
        slot_no;
        "Handled event"
    );
    Ok(())
}

/// Graduates volatile records to persistent storage.
pub fn on_cardano_immutable_roll_forward(
    subscription_id: &SubscriptionId,
    block: &Block,
) -> anyhow::Result<()> {
    let _block = block.to_catalyst_type(subscription_id.get_network());
    let conn = sqlite::Connection::open(false)?;
    let _conn_volatile = sqlite::Connection::open(true)?;

    // Simple mock, propagating slot_no.
    let (slot_no,) = conn
        .prepare("SELECT ?")?
        .query_one_as::<(u64,)>(&[&block.get_slot().try_into()?])?;

    trace!(
        target: "staked_ada_indexer::on_cardano_immutable_roll_forward",
        slot_no;
        "Handled event"
    );
    Ok(())
}
