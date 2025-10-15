//! Hermes RTE events inner implementations.

use shared::{
    bindings::hermes::cardano::{
        self,
        api::{Block, SubscriptionId},
    },
    utils::{
        self,
        log::{error, info},
    },
};

use crate::database::create_tables;

/// Initializes sqlite tables and cardano block subscription.
pub fn init() -> anyhow::Result<()> {
    let mut conn = utils::sqlite::Connection::open(false)?;
    let mut conn_volatile = utils::sqlite::Connection::open(true)?;

    create_tables(&mut conn)?;
    create_tables(&mut conn_volatile)?;

    let subscribe_from = cardano::api::SyncSlot::Genesis;
    let network = cardano::api::CardanoNetwork::Preprod;

    let network_resource = cardano::api::Network::new(network)
        .inspect_err(|error| error!(error:%, network:?; "Failed to create network resource"))?;
    let subscription_id_resource = network_resource
        .subscribe_block(subscribe_from)
        .inspect_err(|error| error!(error:%, subscribe_from:?; "Failed to subscribe block from"))?;

    info!("💫 Network {network:?}, with subscription id: {subscription_id_resource}");

    Ok(())
}

/// Records new transactions.
pub fn on_cardano_block(
    subscription_id: &SubscriptionId,
    block: &Block,
) -> anyhow::Result<()> {
    let _block = block.to_catalyst_type(subscription_id.get_network());
    let sqlite = utils::sqlite::Connection::open(false)?;
    let _sqlite_in_mem = utils::sqlite::Connection::open(true)?;

    let (count,) = sqlite
        .prepare("SELECT COUNT(*) FROM stake_registration")?
        .query_one_as::<(u64,)>(&[])?;

    info!(count, event = "on_cardano_block"; "Total rows in stake_registration persistent table");

    let (count,) = sqlite
        .prepare("SELECT COUNT(*) FROM txi_by_txn_id")?
        .query_one_as::<(u64,)>(&[])?;

    info!(count, event = "on_cardano_block"; "Total rows in txi_by_txn_id persistent table");

    let (count,) = sqlite
        .prepare("SELECT COUNT(*) FROM txo_by_stake_address")?
        .query_one_as::<(u64,)>(&[])?;

    info!(count, event = "on_cardano_block"; "Total rows in txo_by_stake_address persistent table");
    Ok(())
}

/// Graduates volatile records to persistent storage.
pub fn on_cardano_immutable_roll_forward(
    subscription_id: &SubscriptionId,
    block: &Block,
) -> anyhow::Result<()> {
    let _block = block.to_catalyst_type(subscription_id.get_network());
    let _sqlite = utils::sqlite::Connection::open(false)?;
    let sqlite_in_mem = utils::sqlite::Connection::open(true)?;

    let (count,) = sqlite_in_mem
        .prepare("SELECT COUNT(*) FROM stake_registration")?
        .query_one_as::<(u64)>(&[])?;

    info!(
        count, event = "on_cardano_immutable_roll_forward";
        "Total rows in stake_registration persistent table"
    );

    let (count,) = sqlite_in_mem
        .prepare("SELECT COUNT(*) FROM txi_by_txn_id")?
        .query_one_as::<(u64,)>(&[])?;

    info!(
        count, event = "on_cardano_immutable_roll_forward";
        "Total rows in txi_by_txn_id persistent table"
    );

    let (count,) = sqlite_in_mem
        .prepare("SELECT COUNT(*) FROM txo_by_stake_address")?
        .query_one_as::<(u64,)>(&[])?;

    info!(
        count, event = "on_cardano_immutable_roll_forward";
        "Total rows in txo_by_stake_address persistent table"
    );
    Ok(())
}
