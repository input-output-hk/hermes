//! Select from Catalyst ID.

use crate::{
    bind_parameters,
    database::{
        operation::Operation, query_builder::QueryBuilder, select::column_as,
        statement::DatabaseStatement, RBAC_REGISTRATION_PERSISTENT_TABLE_NAME,
        RBAC_REGISTRATION_VOLATILE_TABLE_NAME,
    },
    hermes::sqlite::api::{Sqlite, Statement, StepResult, Value},
    rbac::build_rbac_chain::RbacChainInfo,
    utils::log::log_error,
};

/// Registration chain from a catalyst ID.
/// 
/// Selects a root registration and all its children, returning the full chain given a catalyst ID.
/// If no root registration is found for the given `cat_id`, returns an empty list.
///
/// Root registration:
///
/// The root registration is the registration with matching `catalyst_id` where
/// no `prv_txn_id`, no `problem_report` - valid, least `slot_no`, and least `txn_idx`.
/// In other words, the earliest valid registration containing the given `catalyst_id` is considered the root.
/// Note that the Catalyst ID is derive from the subject public key or Role 0 registration.
/// If the registration chain contains multiple Catalyst IDs (multiple Role 0 subject public keys),
/// the first catalyst ID in the chain is used.
///
/// For example, a chain with the first registration having `catalyst_id_a` and the second registration having `catalyst_id_b`,
/// When requesting for `catalyst_id_b` WILL NOT result in the same chain as requesting for `catalyst_id_a`.
///
/// In addition, if there is an attempt to creating a new chain with `catalyst_id_b`, the chain will be invalid
/// since **IT IS NOT ALLOWED TO USE THE PUBLIC KEYS OF AN EXISTING VALID CHAIN**.
///
///
/// Child registration:
///
/// The child registration is determine by having a `prv_txn_id` pointing back to a parent.
/// In other words, the child registration is the registration with matching `prv_txn_id` where
/// no `problem_report` - valid, least `slot_no`, and least `txn_idx`.
///
/// An update that cause the link to break is considered invalid.
///
/// For example,
///
/// Root    : `txn_id` = `a` |  `prv_txn_id` = null | slot 10  | valid |
/// Child1  : `txn_id` = `b` |  `prv_txn_id` = `a`  | slot 11  | valid |
/// Child2  : `txn_id` = `c` |  `prv_txn_id` = `b`  | slot 12  | invalid |
/// Child3  : `txn_id` = `d` |  `prv_txn_id` = `c`  | slot 13  | valid |
/// Child4  : `txn_id` = `e` |  `prv_txn_id` = `b`  | slot 14  | invalid |
///
/// The valid chain will be Root -> Child1 -> Child4
///
/// # Returns
///
/// * `Ok(Vec<RbacChainInfo>)` – The registration chain associated with the given catalyst ID.
///   If the vector is empty, no chain is found.
/// * `Err(anyhow::Error)` – If any error occurs.
pub(crate) fn select_rbac_registration_chain_from_cat_id(
    sqlite: &Sqlite,
    sqlite_in_mem: &Sqlite,
    cat_id: &str,
) -> anyhow::Result<Vec<RbacChainInfo>> {
    const FUNCTION_NAME: &str = "select_rbac_registration_chain_from_cat_id";

    // --- Find the root ---
    let (mut txn_id, mut chain) = if let Some(r) =
        extract_root(sqlite, cat_id, RBAC_REGISTRATION_PERSISTENT_TABLE_NAME)?
    {
        r
    } else if let Some(r) = extract_root(sqlite_in_mem, cat_id, RBAC_REGISTRATION_VOLATILE_TABLE_NAME)? {
        r
    } else {
        return Ok(vec![]); // no root found
    };

    // --- Find children ---
    let p_stmt = DatabaseStatement::prepare_statement(
        sqlite,
        &QueryBuilder::select_child_reg_from_parent(&RBAC_REGISTRATION_PERSISTENT_TABLE_NAME),
        Operation::Select,
        FUNCTION_NAME,
    )?;

    let v_stmt = DatabaseStatement::prepare_statement(
        sqlite_in_mem,
        &QueryBuilder::select_child_reg_from_parent(&RBAC_REGISTRATION_VOLATILE_TABLE_NAME),
        Operation::Select,
        FUNCTION_NAME,
    )?;
    loop {
        // Try to find child in persistent table first
        // Persistent first
        if let Some((next_txn_id, slot_no, txn_idx)) =
            extract_child(&p_stmt, &RBAC_REGISTRATION_PERSISTENT_TABLE_NAME, &txn_id)?
        {
            txn_id = next_txn_id;
            chain.push(RbacChainInfo { slot_no, txn_idx });
            continue;
        }

        // Then volatile
        match extract_child(&v_stmt, &RBAC_REGISTRATION_VOLATILE_TABLE_NAME, &txn_id)? {
            Some((next_txn_id, slot_no, txn_idx)) => {
                txn_id = next_txn_id;
                chain.push(RbacChainInfo { slot_no, txn_idx });
            },
            // No child found for both persistent and volatile
            None => break,
        }
    }
    DatabaseStatement::finalize_statement(p_stmt, FUNCTION_NAME);
    DatabaseStatement::finalize_statement(v_stmt, FUNCTION_NAME);
    Ok(chain)
}

/// Extract the root registration.
fn extract_root(
    sqlite: &Sqlite,
    cat_id: &str,
    table: &str,
) -> anyhow::Result<Option<(Value, Vec<RbacChainInfo>)>> {
    const FUNCTION_NAME: &str = "extract_root";

    let stmt = DatabaseStatement::prepare_statement(
        sqlite,
        &QueryBuilder::select_root_reg_by_cat_id(table),
        Operation::Select,
        FUNCTION_NAME,
    )?;
    bind_parameters!(stmt, FUNCTION_NAME, cat_id.to_string() => "catalyst_id")?;

    // The first valid root registration is chosen
    let result = match stmt.step() {
        Ok(StepResult::Row) => {
            let txn_id = stmt.column(0)?;
            let slot_no = column_as::<u64>(&stmt, 1, FUNCTION_NAME, "slot_no")?;
            let txn_idx = column_as::<u16>(&stmt, 2, FUNCTION_NAME, "txn_idx")?;
            Some((txn_id.clone(), vec![RbacChainInfo { slot_no, txn_idx }]))
        },
        Ok(StepResult::Done) => None,
        Err(e) => {
            let error = format!("Failed to step in {table}: {e}");
            log_error(
                file!(),
                FUNCTION_NAME,
                "hermes::sqlite::api::step",
                &error,
                None,
            );
            anyhow::bail!(error);
        },
    };

    DatabaseStatement::finalize_statement(stmt, FUNCTION_NAME)?;
    Ok(result)
}

/// Extract the child registration.
fn extract_child(
    stmt: &Statement,
    table: &str,
    txn_id: &Value,
) -> anyhow::Result<Option<(Value, u64, u16)>> {
    const FUNCTION_NAME: &str = "extract_child";

    // Reset first to ensure the statement is in a clean state
    DatabaseStatement::reset_statement(&stmt, FUNCTION_NAME)?;
    bind_parameters!(stmt, FUNCTION_NAME, txn_id.clone() => "txn_id")?;
    let result = match stmt.step() {
        Ok(StepResult::Row) => {
            let next_txn_id = stmt.column(0)?;
            let slot_no = column_as::<u64>(&stmt, 1, FUNCTION_NAME, "slot_no")?;
            let txn_idx = column_as::<u16>(&stmt, 2, FUNCTION_NAME, "txn_idx")?;
            Some((next_txn_id, slot_no, txn_idx))
        },
        Ok(StepResult::Done) => None,
        Err(e) => {
            let error = format!("Failed to step in {table}: {e}");
            log_error(
                file!(),
                FUNCTION_NAME,
                "hermes::sqlite::api::step",
                &error,
                None,
            );
            anyhow::bail!(error);
        },
    };
    Ok(result)
}
