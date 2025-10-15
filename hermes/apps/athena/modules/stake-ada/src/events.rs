//! Hermes RTE events inner implementations.

use shared::{
    bindings::hermes::cardano::{
        self,
        api::{Block, SubscriptionId},
    },
    utils::{
        self,
        log::{error, info, trace},
    },
};

use crate::database::create_tables;

/// Initializes sqlite tables and cardano block subscription.
pub fn init() -> anyhow::Result<()> {
    let mut conn = utils::sqlite::Connection::open(false)?;

    let mut _conn_volatile = utils::sqlite::Connection::open(true)?;

    create_tables(&mut conn)?;

    let subscribe_from = cardano::api::SyncSlot::Genesis;
    let network = cardano::api::CardanoNetwork::Preprod;

    let network_resource = cardano::api::Network::new(network)
        .inspect_err(|error| error!(error:%, network:?; "Failed to create network resource"))?;
    let subscription_id_resource = network_resource
        .subscribe_block(subscribe_from)
        .inspect_err(|error| error!(error:%, subscribe_from:?; "Failed to subscribe block from"))?;

    info!(
        target: "staked_ada::init",
        "💫 Network {network:?}, with subscription id: {subscription_id_resource}"
    );

    Ok(())
}

/// Records new transactions.
pub fn on_cardano_block(
    subscription_id: &SubscriptionId,
    block: &Block,
) -> anyhow::Result<()> {
    let _block = block.to_catalyst_type(subscription_id.get_network());
    let conn = utils::sqlite::Connection::open(false)?;
    let _conn_volatile = utils::sqlite::Connection::open(true)?;

    let (count_stake_registration,) = conn
        .prepare("SELECT COUNT(*) FROM stake_registration")?
        .query_one_as::<(Option<u64>,)>(&[])?;

    let (count_txi_by_txn_id,) = conn
        .prepare("SELECT COUNT(*) FROM txi_by_txn_id")?
        .query_one_as::<(Option<u64>,)>(&[])?;

    let (count_txo_by_stake_address,) = conn
        .prepare("SELECT COUNT(*) FROM txo_by_stake_address")?
        .query_one_as::<(Option<u64>,)>(&[])?;

    trace!(
        target: "staked_ada::on_cardano_block",
        count_stake_registration,
        count_txi_by_txn_id,
        count_txo_by_stake_address;
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
    let conn = utils::sqlite::Connection::open(false)?;
    let _conn_volatile = utils::sqlite::Connection::open(true)?;

    let (count_stake_registration,) = conn
        .prepare("SELECT COUNT(*) FROM stake_registration")?
        .query_one_as::<(Option<u64>,)>(&[])?;

    let (count_txi_by_txn_id,) = conn
        .prepare("SELECT COUNT(*) FROM txi_by_txn_id")?
        .query_one_as::<(Option<u64>,)>(&[])?;

    let (count_txo_by_stake_address,) = conn
        .prepare("SELECT COUNT(*) FROM txo_by_stake_address")?
        .query_one_as::<(Option<u64>,)>(&[])?;

    trace!(
        target: "staked_ada::on_cardano_immutable_roll_forward",
        count_stake_registration,
        count_txi_by_txn_id,
        count_txo_by_stake_address;
        "Handled event"
    );
    Ok(())
}
