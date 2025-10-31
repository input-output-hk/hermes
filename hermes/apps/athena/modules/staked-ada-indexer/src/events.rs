//! Hermes RTE events inner implementations.

use anyhow::Context;
use shared::{
    bindings::hermes::cardano::{
        self,
        api::{Block, SubscriptionId},
    },
    database::staked_ada::{
        create_tables, delete_stake_registration_before_slot, delete_stake_registration_since_slot,
        delete_txi_before_slot, delete_txi_since_slot, delete_txo_assets_before_slot,
        delete_txo_assets_since_slot, delete_txo_before_slot, delete_txo_since_slot,
        insert_txi_by_txn_id, insert_txo_assets_by_stake, insert_txo_by_stake,
    },
    utils::{
        log::{info, trace},
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
            tx.execute(q).context("Failed to execute init sql query")?;
        }
        tx.commit()?;
    }

    info!(target: "staked_ada_indexer::init", "💫 Sqlite initialized.");

    if !config::OFFLINE {
        info!(target: "staked_ada_indexer::init", "💫 Setting up Cardano subscription...");

        let network = cardano::api::CardanoNetwork::Preprod;

        let network_resource = cardano::api::Network::new(network)?;
        let subscription_id_resource = network_resource.subscribe_block(config::SUBSCRIBE_FROM)?;

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
    trace!(
        target: "staked_ada_indexer::on_cardano_block",
        slot_no = block.get_slot(),
        is_immutable = block.is_immutable();
        "💫 Handling cardano block..."
    );

    if block.is_rollback()? {
        trace!(
            target: "staked_ada_indexer::on_cardano_block",
            "💫 Block is the first block of a rollback. Removing volatile database records..."
        );

        let mut conn = sqlite::Connection::open(true)?;
        let mut tx = conn.begin()?;
        delete_stake_registration_since_slot(&mut tx, block.get_slot())?;
        delete_txi_since_slot(&mut tx, block.get_slot())?;
        delete_txo_since_slot(&mut tx, block.get_slot())?;
        delete_txo_assets_since_slot(&mut tx, block.get_slot())?;
        tx.commit()?;

        trace!(
            target: "staked_ada_indexer::on_cardano_block",
            slot_no = block.get_slot(),
            is_immutable = block.is_immutable();
            "💫 Volatile database records removed. Rollback handled"
        );
    }

    let block = block.to_catalyst_type(subscription_id.get_network())?;
    let mut conn = sqlite::Connection::open(!block.is_immutable())?;

    trace!(
        target: "staked_ada_indexer::on_cardano_block",
        "💫 Indexing block..."
    );

    let mut buffers = index::Buffers::default();
    buffers.index_block(&block);

    trace!(
        target: "staked_ada_indexer::on_cardano_block",
        "💫 Block is indexed. Inserting block data into database..."
    );

    // Assume everything is broken if one of the inserts fails.
    let mut sql_tx = conn.begin()?;
    insert_txo_by_stake(&mut sql_tx, buffers.txo_by_stake).map_err(|(_, e)| e)?;
    insert_txo_assets_by_stake(&mut sql_tx, buffers.txo_assets_by_stake).map_err(|(_, e)| e)?;
    insert_txi_by_txn_id(&mut sql_tx, buffers.txi_by_txn_id).map_err(|(_, e)| e)?;
    sql_tx.commit()?;

    trace!(
        target: "staked_ada_indexer::on_cardano_block",
        "💫 Block data is inserted. Handled event"
    );

    Ok(())
}

/// Graduates volatile records to persistent storage.
pub fn on_cardano_immutable_roll_forward(
    subscription_id: &SubscriptionId,
    block: &Block,
) -> anyhow::Result<()> {
    trace!(
        target: "staked_ada_indexer::on_cardano_immutable_roll_forward",
        slot_no = block.get_slot(),
        is_immutable = block.is_immutable();
        "💫 Handling immutable roll forward..."
    );

    let network_resource = cardano::api::Network::new(subscription_id.get_network())?;
    let Some((immutable, mutable)) = network_resource.get_tips() else {
        anyhow::bail!("Failed to get tips");
    };

    // Only process immutable roll forward when it reaches the tip.
    // In case a block is not at the tip, do nothing.
    if mutable != block.get_slot() {
        trace!(
            target: "staked_ada_indexer::on_cardano_immutable_roll_forward",
            "💫 Block is not at the tip – skipping. Handled event."
        );
        return Ok(());
    }

    trace!(
        target: "staked_ada_indexer::on_cardano_immutable_roll_forward",
        "💫 Updating block subscription..."
    );

    network_resource.subscribe_block(cardano::api::SyncSlot::Specific(immutable))?;
    subscription_id.unsubscribe();

    trace!(
        target: "staked_ada_indexer::on_cardano_immutable_roll_forward",
        "💫 Subscription updated. Removing volatile database records..."
    );

    let mut conn = sqlite::Connection::open(true)?;

    let mut tx = conn.begin()?;
    delete_stake_registration_before_slot(&mut tx, block.get_slot())?;
    delete_txo_before_slot(&mut tx, block.get_slot())?;
    delete_txo_assets_before_slot(&mut tx, block.get_slot())?;
    delete_txi_before_slot(&mut tx, block.get_slot())?;
    tx.commit()?;

    trace!(
        target: "staked_ada_indexer::on_cardano_immutable_roll_forward",
        "💫 Volatile data removed. Handled event"
    );
    Ok(())
}
