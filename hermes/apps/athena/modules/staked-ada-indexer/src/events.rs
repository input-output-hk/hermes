//! Hermes RTE events inner implementations.

use shared::{
    bindings::hermes::cardano::{
        self,
        api::{Block, SubscriptionId},
    },
    database::staked_ada::{
        create_tables, insert_txi_by_txn_id, insert_txo_assets_by_stake, insert_txo_by_stake,
    },
    utils::{
        log::{error, info, trace},
        sqlite,
    },
};

use crate::{config, index};

/// Initializes sqlite tables and cardano block subscription.
pub fn init() -> anyhow::Result<()> {
    info!(target: "staked_ada_indexer::init", "💫 Initializing Sqlite...");

    for in_mem in [false, true] {
        let mut conn = sqlite::Connection::open(in_mem)?;
        let mut tx = conn.begin()?;
        create_tables(&mut tx)?;
        if let Some(q) = config::INIT_SQL_QUERY {
            tx.execute(q)
                .inspect_err(|error| error!(error:%; "Failed to execute init sql query"))?;
        }
        tx.commit()?;
    }

    info!(target: "staked_ada_indexer::init", "💫 Sqlite initialized.");

    if !config::OFFLINE {
        info!(target: "staked_ada_indexer::init", "💫 Setting up Cardano subscription...");

        let network = cardano::api::CardanoNetwork::Preprod;

        let network_resource = cardano::api::Network::new(network)
            .inspect_err(|error| error!(error:%, network:?; "Failed to create network resource"))?;
        let subscription_id_resource = network_resource
            .subscribe_block(config::SUBSCRIBE_FROM)
            .inspect_err(|error| {
                error!(
                    error:%,
                    subscribe_from:? = config::SUBSCRIBE_FROM;
                    "Failed to subscribe block from"
                );
            })?;

        info!(
            target: "staked_ada_indexer::init",
            network:?,
            subscription_id_resource:?;
            "💫 Cardano subscription set up."
        );
    }

    Ok(())
}

/// Records new transactions.
pub fn on_cardano_block(
    subscription_id: &SubscriptionId,
    block: &Block,
) -> anyhow::Result<()> {
    let block = block.to_catalyst_type(subscription_id.get_network())?;
    let mut conn = sqlite::Connection::open(!block.is_immutable())?;

    trace!(
        target: "staked_ada_indexer::on_cardano_block",
        slot_no = u64::from(block.slot()),
        is_immutable = block.is_immutable();
        "Indexing block..."
    );

    let mut buffers = index::Buffers::default();
    buffers.index_block(&block);

    trace!(
        target: "staked_ada_indexer::on_cardano_block",
        slot_no = u64::from(block.slot());
        "Block is indexed. Inserting block data into database..."
    );

    // Assume everything is broken if one of the inserts fails.
    let mut sql_tx = conn.begin()?;
    insert_txo_by_stake(&mut sql_tx, buffers.txo_by_stake).map_err(|(_, error)| {
        error!(
            target: "staked_ada_indexer::on_cardano_block",
            error:%,
            slot_no = u64::from(block.slot());
            "Failed to insert txo by stake");
        error
    })?;
    insert_txo_assets_by_stake(&mut sql_tx, buffers.txo_assets_by_stake).map_err(
        |(_, error)| {
            error!(
                target: "staked_ada_indexer::on_cardano_block",
                error:%,
                slot_no = u64::from(block.slot());
                "Failed to insert txo assets by stake");
            error
        },
    )?;
    insert_txi_by_txn_id(&mut sql_tx, buffers.txi_by_txn_id).map_err(|(_, error)| {
        error!(
            target: "staked_ada_indexer::on_cardano_block",
            error:%,
            slot_no = u64::from(block.slot());
            "Failed to insert txi by txn id");
        error
    })?;
    sql_tx.commit()?;

    trace!(
        target: "staked_ada_indexer::on_cardano_block",
        slot_no = u64::from(block.slot());
        "Block data is inserted. Handled event"
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
