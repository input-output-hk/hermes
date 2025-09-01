use serde_json::json;

use crate::{
    database::{
        data::{rbac_db::RbacDbData, rbac_stake_db::RbacStakeDbData},
        SQLITE,
    },
    hermes::{
        self,
        hermes::sqlite::api::{Statement, Value},
    },
    utils::log::{log_error, log_info},
};

const FILE_NAME: &str = "rbac-registration/src/database/insert.rs";

pub(crate) const RBAC_INSERT_RBAC_REGISTRATION: &str = r#"
    INSERT INTO rbac_registration (
        txn_id, slot_no, txn_idx, prv_txn_id, purpose, catalyst_id, problem_report
    )
    VALUES(?, ?, ?, ?, ?, ?, ?);
"#;

pub(crate) const RBAC_INSERT_STAKE_ADDRESS: &str = r#"
    INSERT INTO rbac_stake_address (
        stake_address, slot_no, txn_index, catalyst_id
    )
    VALUES(?, ?, ?, ?);
"#;

pub(crate) fn prepare_insert_rbac_registration() -> Option<Statement> {
    const FUNCTION_NAME: &str = "prepare_insert_rbac_registration";
    log_info(
        FILE_NAME,
        FUNCTION_NAME,
        "",
        &format!("Prepare insert 🍊"),
        None,
    );
    SQLITE
        .prepare(RBAC_INSERT_RBAC_REGISTRATION)
        .map_err(|e| {
            log_error(
                FILE_NAME,
                FUNCTION_NAME,
                "hermes::sqlite::api::prepare",
                &format!("🚨 Failed to prepare insert statement: {e}"),
                None,
            );
        })
        .ok()
}

pub(crate) fn insert_rbac_registration(
    stmt: &Statement,
    data: RbacDbData,
) {
    const FUNCTION_NAME: &str = "insert_rbac_registration";
    log_error(FILE_NAME, FUNCTION_NAME, "", &format!("Insert 🍊"), None);

    bind_rbac_registration(stmt, data);
    if let Err(e) = stmt.step() {
        log_error(
            FILE_NAME,
            FUNCTION_NAME,
            "hermes::sqlite::api::step",
            &format!("🚨 Failed to step: {e:?}"),
            None,
        )
    }
    if let Err(e) = stmt.reset() {
        log_error(
            FILE_NAME,
            FUNCTION_NAME,
            "hermes::sqlite::api::reset",
            &format!("🚨 Failed to reset: {e:?}"),
            None,
        )
    }
}

fn bind_rbac_registration(
    stmt: &Statement,
    data: RbacDbData,
) {
    const FUNCTION_NAME: &str = "bind_rbac_registration";

    bind_with_log(stmt, FUNCTION_NAME, 1, &data.txn_id.into(), "txn_id");
    bind_with_log(
        stmt,
        FUNCTION_NAME,
        2,
        &data.catalyst_id.into(),
        "catalyst_id",
    );
    bind_with_log(stmt, FUNCTION_NAME, 3, &data.slot.into(), "slot");
    bind_with_log(stmt, FUNCTION_NAME, 4, &data.txn_idx.into(), "txn_idx");
    bind_with_log(
        stmt,
        FUNCTION_NAME,
        5,
        &data.prv_txn_id.into(),
        "prv_txn_id",
    );
    bind_with_log(stmt, FUNCTION_NAME, 6, &data.purpose.into(), "purpose");
    bind_with_log(
        stmt,
        FUNCTION_NAME,
        7,
        &data.problem_report.into(),
        "problem_report",
    );
}

pub(crate) fn prepare_insert_rbac_stake_address() -> anyhow::Result<Statement> {
    const FUNCTION_NAME: &str = "prepare_insert_rbac_stake_address";
    // log_info(
    //     FILE_NAME,
    //     FUNCTION_NAME,
    //     "",
    //     &format!("Prepare insert 🍊"),
    //     None,
    // );

    SQLITE.prepare(RBAC_INSERT_STAKE_ADDRESS).map_err(|e| {
        log_error(
            FILE_NAME,
            FUNCTION_NAME,
            "hermes::sqlite::api::prepare",
            &format!("🚨 Failed to prepare insert: {e}"),
            None,
        );
        anyhow::anyhow!(e)
    })
}

pub(crate) fn insert_rbac_stake_address(
    stmt: &Statement,
    data: RbacStakeDbData,
) {
    const FUNCTION_NAME: &str = "insert_rbac_stake_address";
    // log_info(FILE_NAME, FUNCTION_NAME, "", &format!("Insert 🍊"), None);

    bind_rbac_stake_address(stmt, data);
    if let Err(e) = stmt.step() {
        log_error(
            FILE_NAME,
            FUNCTION_NAME,
            "hermes::sqlite::api::step",
            &format!("🚨 Failed to step: {e:?}"),
            None,
        )
    }
    if let Err(e) = stmt.reset() {
        log_error(
            FILE_NAME,
            FUNCTION_NAME,
            "hermes::sqlite::api::reset",
            &format!("🚨 Failed to reset: {e:?}"),
            None,
        )
    }
}

fn bind_rbac_stake_address(
    stmt: &Statement,
    data: RbacStakeDbData,
) {
    const FUNCTION_NAME: &str = "bind_rbac_stake_address";
    bind_with_log(
        stmt,
        FUNCTION_NAME,
        1,
        &data.stake_address.into(),
        "stake_address",
    );
    bind_with_log(stmt, FUNCTION_NAME, 2, &data.slot.into(), "slot");
    bind_with_log(stmt, FUNCTION_NAME, 3, &data.txn_idx.into(), "txn_idx");
    bind_with_log(
        stmt,
        FUNCTION_NAME,
        4,
        &data.catalyst_id.into(),
        "catalyst_id",
    );
}

// --------------- Binding helper -------------------
fn bind_with_log(
    stmt: &Statement,
    func_name: &str,
    idx: u32,
    value: &Value,
    field_name: &str,
) {
    if let Err(e) = stmt.bind(idx, value) {
        log_error(
            FILE_NAME,
            func_name,
            "hermes::sqlite::bind",
            &format!("🚨 Failed to bind: {e:?}"),
            Some(&json!({ field_name: format!("{value:?}") }).to_string()),
        );
    }
}
