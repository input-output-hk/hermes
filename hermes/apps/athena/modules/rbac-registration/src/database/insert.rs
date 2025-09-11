//! RBAC registration database insert.

use crate::{
    database::{
        bind_with_log,
        data::{rbac_db::RbacDbData, rbac_stake_db::RbacStakeDbData},
    },
    hermes::hermes::sqlite::api::{Sqlite, Statement, Value},
    utils::log::log_error,
};

/// Insert data to `rbac_registration` table.
pub(crate) const RBAC_INSERT_RBAC_REGISTRATION: &str = r#"
    INSERT INTO rbac_registration (
        txn_id, slot_no, txn_idx, prv_txn_id, purpose, catalyst_id, problem_report
    )
    VALUES(?, ?, ?, ?, ?, ?, ?);
"#;

/// Insert data to `rbac_stake_address` table.
pub(crate) const RBAC_INSERT_STAKE_ADDRESS: &str = r#"
    INSERT INTO rbac_stake_address (
        stake_address, slot_no, txn_idx, catalyst_id, txn_id
    )
    VALUES(?, ?, ?, ?, ?);
"#;

/// Prepare insert statement for `rbac_registration` table.
pub(crate) fn prepare_insert_rbac_registration(sqlite: &Sqlite) -> anyhow::Result<Statement> {
    const FUNCTION_NAME: &str = "prepare_insert_rbac_registration";
    match sqlite.prepare(RBAC_INSERT_RBAC_REGISTRATION) {
        Ok(stmt) => Ok(stmt),
        Err(e) => {
            let err_msg = "Failed to prepare insert statement";
            log_error(
                file!(),
                FUNCTION_NAME,
                "hermes::sqlite::api::prepare",
                &format!("{err_msg}: {e}"),
                None,
            );
            anyhow::bail!(err_msg)
        },
    }
}

/// Insert data to `rbac_registration` table.
pub(crate) fn insert_rbac_registration(
    stmt: &Statement,
    data: RbacDbData,
) {
    const FUNCTION_NAME: &str = "insert_rbac_registration";

    bind_rbac_registration(stmt, data);
    if let Err(e) = stmt.step() {
        log_error(
            file!(),
            FUNCTION_NAME,
            "hermes::sqlite::api::step",
            &format!("Failed to step: {e:?}"),
            None,
        )
    }
    if let Err(e) = stmt.reset() {
        log_error(
            file!(),
            FUNCTION_NAME,
            "hermes::sqlite::api::reset",
            &format!("Failed to reset: {e:?}"),
            None,
        )
    }
}

/// Bind data to `rbac_registration` table.
fn bind_rbac_registration(
    stmt: &Statement,
    data: RbacDbData,
) {
    const FUNCTION_NAME: &str = "bind_rbac_registration";

    // Try to convert slot safely, if fail exit the function so no binding is done.
    let slot: Value = match data.slot.try_into() {
        Ok(s) => s,
        Err(e) => {
            log_error(
                file!(),
                FUNCTION_NAME,
                "slot.try_into()",
                &format!("Failed to convert slot: {e}"),
                None,
            );
            return;
        },
    };

    bind_with_log(stmt, FUNCTION_NAME, 1, &data.txn_id.into(), "txn_id");
    bind_with_log(stmt, FUNCTION_NAME, 2, &slot, "slot_no");
    bind_with_log(stmt, FUNCTION_NAME, 3, &data.txn_idx.into(), "txn_idx");
    bind_with_log(
        stmt,
        FUNCTION_NAME,
        4,
        &data.prv_txn_id.into(),
        "prv_txn_id",
    );
    bind_with_log(stmt, FUNCTION_NAME, 5, &data.purpose.into(), "purpose");
    bind_with_log(
        stmt,
        FUNCTION_NAME,
        6,
        &data.catalyst_id.map(|id| id.trim().to_string()).into(),
        "catalyst_id",
    );

    bind_with_log(
        stmt,
        FUNCTION_NAME,
        7,
        &data.problem_report.into(),
        "problem_report",
    );
}

/// Prepare insert statement for `rbac_stake_address` table.
pub(crate) fn prepare_insert_rbac_stake_address(sqlite: &Sqlite) -> anyhow::Result<Statement> {
    const FUNCTION_NAME: &str = "prepare_insert_rbac_stake_address";

    match sqlite.prepare(RBAC_INSERT_STAKE_ADDRESS) {
        Ok(stmt) => Ok(stmt),
        Err(e) => {
            let err_msg = "Failed to prepare insert statement";
            log_error(
                file!(),
                FUNCTION_NAME,
                "hermes::sqlite::api::prepare",
                &format!("{err_msg}: {e}"),
                None,
            );
            anyhow::bail!(err_msg)
        },
    }
}

/// Insert data to `rbac_stake_address` table.
pub(crate) fn insert_rbac_stake_address(
    stmt: &Statement,
    data: RbacStakeDbData,
) {
    const FUNCTION_NAME: &str = "insert_rbac_stake_address";

    bind_rbac_stake_address(stmt, data);
    if let Err(e) = stmt.step() {
        log_error(
            file!(),
            FUNCTION_NAME,
            "hermes::sqlite::api::step",
            &format!("Failed to step: {e:?}"),
            None,
        )
    }
    if let Err(e) = stmt.reset() {
        log_error(
            file!(),
            FUNCTION_NAME,
            "hermes::sqlite::api::reset",
            &format!("Failed to reset: {e:?}"),
            None,
        )
    }
}

/// Bind data to `rbac_stake_address` table.
fn bind_rbac_stake_address(
    stmt: &Statement,
    data: RbacStakeDbData,
) {
    const FUNCTION_NAME: &str = "bind_rbac_stake_address";

    // Try to convert slot safely, if fail exit the function so no binding is done.
    let slot: Value = match data.slot.try_into() {
        Ok(s) => s,
        Err(e) => {
            log_error(
                file!(),
                FUNCTION_NAME,
                "slot.try_into()",
                &format!("Failed to convert slot: {e}"),
                None,
            );
            return;
        },
    };

    bind_with_log(
        stmt,
        FUNCTION_NAME,
        1,
        &data.stake_address.into(),
        "stake_address",
    );
    bind_with_log(stmt, FUNCTION_NAME, 2, &slot, "slot_no");
    bind_with_log(stmt, FUNCTION_NAME, 3, &data.txn_idx.into(), "txn_idx");
    bind_with_log(
        stmt,
        FUNCTION_NAME,
        4,
        &data.catalyst_id.map(|id| id.trim().to_string()).into(),
        "catalyst_id",
    );
    bind_with_log(stmt, FUNCTION_NAME, 5, &data.txn_id.into(), "txn_id");
}
