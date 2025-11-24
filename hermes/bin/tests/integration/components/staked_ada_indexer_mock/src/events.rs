//! Hermes RTE events inner implementations.

use cardano_blockchain_types::hashes::{Blake2bHash, TransactionId};
use shared::{
    database::staked_ada::{TxoByStakeRow, create_tables, insert_txo_by_stake},
    utils::{common::types::cardano::cip19_stake_address::Cip19StakeAddress, log::info, sqlite},
};

/// Mock stake address used for testing
// cspell:ignore racwpyrnngpzvjfcf dacpvd djgfkd cfzwyau
const MOCKED_STAKE_ADDRESS: &str = "stake1ux5wm486ud2racwpyrnngpzvjfcf839dacpvd60djgfkd0cfzwyau";

/// Initializes sqlite tables and cardano block subscription.
pub fn init() -> anyhow::Result<()> {
    info!(
        target: "staked_ada_indexer::init",
        "💫 Initializing Sqlite..."
    );

    let mut conn = sqlite::Connection::open(false)?;
    let mut conn_volatile = sqlite::Connection::open(true)?;

    create_tables(&mut conn)?;
    create_tables(&mut conn_volatile)?;

    insert_test_data(&mut conn)?;
    insert_test_data(&mut conn_volatile)?;

    info!(
        target: "staked_ada_indexer::init",
        "💫 Sqlite initialized."
    );

    Ok(())
}

/// Inserts test data for the mocked stake address.
fn insert_test_data(conn: &mut sqlite::Connection) -> anyhow::Result<()> {
    let cip19_stake = Cip19StakeAddress::try_from(MOCKED_STAKE_ADDRESS)?;
    let stake_address: cardano_blockchain_types::StakeAddress = cip19_stake.try_into()?;
    let txn_id: TransactionId = Blake2bHash::from([0u8; 32]).into();

    let txo_rows = vec![TxoByStakeRow {
        stake_address: stake_address.clone(),
        txn_id,
        txn_index: 0,
        txo: 0,
        slot_no: 12345,
        value: 100_000_000u64.into(),
        spent_slot: None,
    }];

    let count = insert_txo_by_stake(conn, txo_rows)
        .map_err(|(_, err)| anyhow::anyhow!("Failed to insert txo_by_stake: {err:?}"))?;

    info!(
        target: "staked_ada_indexer::init",
        "💫 Test data inserted for stake address: {MOCKED_STAKE_ADDRESS}, rows inserted: {count}"
    );

    Ok(())
}
