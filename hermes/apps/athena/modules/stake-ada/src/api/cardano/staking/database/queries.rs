use std::sync::{Arc, RwLock};

use anyhow::Context as _;
use cardano_blockchain_types::{hashes::TransactionId, StakeAddress};
use shared::{
    bindings::hermes::sqlite::api::Sqlite,
    utils::sqlite::{operation::Operation, statement::DatabaseStatement},
};

use crate::api::cardano::staking::database::types::{
    DbSlot, DbStakeAddress, DbTransactionId, DbTxnIndex, DbTxnOutputOffset, DbValue,
    GetAssetsByStakeAddressQuery, GetAssetsByStakeAddressQueryKey,
    GetAssetsByStakeAddressQueryValue, GetTxiByTxnHashesQuery, GetTxoByStakeAddressQuery,
    GetTxoByStakeAddressQueryKey, GetTxoByStakeAddressQueryValue, UpdateTxoSpentQueryParams,
};

pub(crate) fn get_txi_by_txn_hashes(
    session: &Sqlite,
    txn_ids: &[TransactionId],
) -> anyhow::Result<Vec<GetTxiByTxnHashesQuery>> {
    let placeholders = (0..txn_ids.len())
        .map(|_| "?")
        .collect::<Vec<_>>()
        .join(",");

    let query = format!(
        r#"
        SELECT
            txn_id,
            txo,
            slot_no
        FROM txn_txi_id
        WHERE txn_id IN ({})
        "#,
        placeholders
    );

    let statement = DatabaseStatement::prepare_statement(
        session,
        &query,
        Operation::Select,
        "get_txi_by_txn_hashes",
    )?;
    for (bind_id, txn_id) in txn_ids.iter().enumerate() {
        let db_txn_id: DbTransactionId = (*txn_id).into();
        let bind_id = bind_id
            .checked_add(1)
            .with_context(|| "binding index overflowed".to_string())?
            .try_into()
            .with_context(|| "conversion to u32 failed".to_string())?;
        statement.bind(bind_id, &db_txn_id.into())?;
    }

    let rows = DatabaseStatement::select_all::<(DbTransactionId, DbTxnOutputOffset, DbSlot)>(
        &statement,
        "get_txi_by_txn_hashes",
    )?
    .into_iter()
    .map(|(txn_id, txo, slot_no)| GetTxiByTxnHashesQuery {
        txn_id,
        txo,
        slot_no,
    })
    .collect();
    Ok(rows)
}

pub(crate) fn get_txo_by_stake_address(
    session: &Sqlite,
    stake_address: &StakeAddress,
) -> anyhow::Result<Vec<GetTxoByStakeAddressQuery>> {
    let query = r#"
        SELECT
            txn_id,
            txn_index,
            txo,
            slot_no,
            value,
            spent_slot
        FROM txo_by_stake
        WHERE stake_address = ?
    "#;

    let statement = DatabaseStatement::prepare_statement(
        session,
        query,
        Operation::Select,
        "get_txo_by_stake_address",
    )?;

    let db_stake_address: DbStakeAddress = stake_address.clone().into();
    statement
        .bind(1, &db_stake_address.into())
        .with_context(|| "Failed to bind stake_address")?;

    let rows = DatabaseStatement::select_all::<(
        DbTransactionId,
        DbTxnIndex,
        DbTxnOutputOffset,
        DbSlot,
        DbValue,
        // TODO: check if `Option` is correctly parsed
        Option<DbSlot>,
    )>(&statement, "get_txo_by_stake_address")?
    .into_iter()
    .map(|(txn_id, txn_index, txo, slot_no, value, spent_slot)| {
        let key = Arc::new(GetTxoByStakeAddressQueryKey {
            txn_index,
            txo,
            slot_no,
        });
        let value = Arc::new(RwLock::new(GetTxoByStakeAddressQueryValue {
            txn_id: txn_id.into(),
            value: value.into(),
            spent_slot,
        }));
        GetTxoByStakeAddressQuery { key, value }
    })
    .collect();

    Ok(rows)
}

pub(crate) fn get_assets_by_stake_address(
    session: &Sqlite,
    stake_address: &StakeAddress,
) -> anyhow::Result<Vec<GetAssetsByStakeAddressQuery>> {
    let query = r#"
        SELECT
            txn_index,
            txo,
            slot_no,
            policy_id,
            asset_name,
            value
        FROM txo_assets_by_stake
        WHERE stake_address = ?
    "#;

    let statement = DatabaseStatement::prepare_statement(
        session,
        query,
        Operation::Select,
        "get_assets_by_stake_address",
    )?;

    let db_stake_address: DbStakeAddress = stake_address.clone().into();
    statement
        .bind(0, &db_stake_address.into())
        .with_context(|| "Failed to bind stake_address")?;

    let rows = DatabaseStatement::select_all::<(
        DbTxnIndex,
        DbTxnOutputOffset,
        DbSlot,
        Vec<u8>,
        Vec<u8>,
        DbValue,
    )>(&statement, "get_assets_by_stake_address")?
    .into_iter()
    .map(|(txn_index, txo, slot_no, policy_id, asset_name, value)| {
        let key = Arc::new(GetAssetsByStakeAddressQueryKey {
            txn_index,
            txo,
            slot_no,
        });
        let value = Arc::new(GetAssetsByStakeAddressQueryValue {
            policy_id,
            asset_name,
            value: value.into(),
        });
        GetAssetsByStakeAddressQuery { key, value }
    })
    .collect();

    Ok(rows)
}

// TODO: update to be batched and inside tx.
pub(crate) fn update_txo_spent(
    session: &Sqlite,
    params: Vec<UpdateTxoSpentQueryParams>,
) -> anyhow::Result<()> {
    let query = r#"
        UPDATE txo_by_stake
            SET spent_slot = ?
        WHERE stake_address = ?
            AND txn_index = ?
            AND txo = ?
            AND slot_no = ?
    "#;

    let statement = DatabaseStatement::prepare_statement(
        session,
        query,
        Operation::Update,
        "update_txo_spent",
    )?;

    for param in params {
        DatabaseStatement::bind_step_reset_statement(
            &statement,
            |statement| {
                statement.bind(1, &param.spent_slot.try_into()?)?;
                statement.bind(2, &param.stake_address.into())?;
                statement.bind(3, &param.txn_index.into())?;
                statement.bind(4, &param.txo.into())?;
                statement.bind(5, &param.slot_no.try_into()?)?;
                Ok(())
            },
            "update_txo_spent",
        )
        .context("Failed to execute update_txo_spent")?;
    }

    Ok(())
}
