//! Select stake address.

use cardano_blockchain_types::StakeAddress;

use crate::{
    bind_parameters,
    database::{
        operation::Operation,
        query_builder::QueryBuilder,
        select::{cat_id::select_rbac_registration_chain_from_cat_id, column_as},
        statement::DatabaseStatement,
        RBAC_REGISTRATION_PERSISTENT_TABLE_NAME, RBAC_REGISTRATION_VOLATILE_TABLE_NAME,
        RBAC_STAKE_ADDRESS_PERSISTENT_TABLE_NAME, RBAC_STAKE_ADDRESS_VOLATILE_TABLE_NAME,
    },
    hermes::sqlite::api::{Sqlite, Statement, StepResult, Value},
    rbac::build_rbac_chain::RbacChainInfo,
    utils::log::log_error,
};

/// Registration chain from a stake address:
///
/// 1. Start from the newest registration.
/// - The rule is the newest `stake_address` that is in a valid chain, will take over the `stake_address` in the older chain.
/// For example, if `stake_address_A` is in a valid chain1 with slot = 10, another valid chain2 with slot 20 has `stake_address_A`.
/// Now `stake_address_A` belong to chain2. because it is the latest one.
///
///      SELECT txn_id
///      FROM rbac_stake_address
///      WHERE stake_address = ?
///      ORDER BY slot_no DESC, txn_idx DESC;
///
/// Produces [txn_id_1, txn_id_2, …], newest to oldest.
///
/// Then there will be 2 cases
///
/// a. Root registration (`prv_txn_id` IS NULL):
/// - A valid root must have a non-null catalyst_id, no problem_report, and no `prv_txn_id`.
/// - Then need to validate by checking whether this root (Catalyst ID) was already used
///   in an earlier root (slot_no less than current slot, or same slot with smaller `txn_idx`).
///
///      SELECT txn_id FROM rbac_registration
///      WHERE prv_txn_id IS NULL
///      AND problem_report IS NULL
///      AND catalyst_id = ?
///      AND (
///          slot_no < ? OR (slot_no = ? AND txn_idx < ?)
///      )  
///
/// - If no earlier root exists -> valid, the given `stake_address` belongs this chain.
/// - Otherwise -> invalid, continue with next `txn_id`.
///
/// b. Non-root registration (`prv_txn_id` IS NOT NULL):
/// - Follow the chain backwards (`txn_id` -> `prv_txn_id` -> …) until reaching a root (`prv_txn_id` IS NULL).
/// - Validate that root using the same rule as (a).
/// - If root is valid -> stake_address belongs to this chain.
/// - Otherwise -> continue with next `txn_id`.
///
/// 3. If no valid root/chain is found after checking all candidates, then `stake_address` does not belong to any valid registration
/// and will return an empty vector.
pub(crate) fn select_rbac_registration_chain_from_stake_addr(
    sqlite: &Sqlite,
    sqlite_in_mem: &Sqlite,
    stake_addr: StakeAddress,
) -> anyhow::Result<Vec<RbacChainInfo>> {
    const FUNCTION_NAME: &str = "select_rbac_registration_chain_from_stake_addr";

    // Convert the given stake address to Vec<u8> which will be use in the query
    let stake: Vec<u8> = stake_addr.try_into().map_err(|e| {
        let error = format!("Failed to convert stake address StakeAddress: {e:?}");
        log_error(file!(), FUNCTION_NAME, "stake_addr.try_into", &error, None);
        anyhow::anyhow!(error)
    })?;

    // List of transaction IDs that contain the given stake address, newest first
    let mut txn_ids = get_txn_ids_from_stake_addr(
        &stake,
        sqlite_in_mem,
        RBAC_STAKE_ADDRESS_VOLATILE_TABLE_NAME,
    )?;
    txn_ids.extend(get_txn_ids_from_stake_addr(
        &stake,
        sqlite,
        RBAC_STAKE_ADDRESS_PERSISTENT_TABLE_NAME,
    )?);

    // --- Prepare statements ---
    let reg_p_stmt = DatabaseStatement::prepare_statement(
        sqlite,
        &QueryBuilder::select_reg_by_txn_id(RBAC_REGISTRATION_PERSISTENT_TABLE_NAME),
        Operation::Select,
        FUNCTION_NAME,
    )?;
    let reg_v_stmt = DatabaseStatement::prepare_statement(
        sqlite_in_mem,
        &QueryBuilder::select_reg_by_txn_id(RBAC_REGISTRATION_VOLATILE_TABLE_NAME),
        Operation::Select,
        FUNCTION_NAME,
    )?;

    let root_validate_p_stmt = DatabaseStatement::prepare_statement(
        sqlite,
        &QueryBuilder::select_root_reg_by_cat_id_less_than_slot_txn_idx(
            RBAC_REGISTRATION_PERSISTENT_TABLE_NAME,
        ),
        Operation::Select,
        FUNCTION_NAME,
    )?;
    let root_validate_v_stmt = DatabaseStatement::prepare_statement(
        sqlite_in_mem,
        &QueryBuilder::select_root_reg_by_cat_id_less_than_slot_txn_idx(
            RBAC_REGISTRATION_VOLATILE_TABLE_NAME,
        ),
        Operation::Select,
        FUNCTION_NAME,
    )?;

    for cur_txn in txn_ids {
        // Reset first to ensure the statement is in a clean state
        DatabaseStatement::reset_statement(&reg_p_stmt, FUNCTION_NAME)?;
        DatabaseStatement::reset_statement(&reg_v_stmt, FUNCTION_NAME)?;
        DatabaseStatement::reset_statement(&root_validate_p_stmt, FUNCTION_NAME)?;
        DatabaseStatement::reset_statement(&root_validate_v_stmt, FUNCTION_NAME)?;

        // Get registration information, trying persistent first, then volatile
        let registration_info = get_registration_info_from_txn_id(&reg_p_stmt, &cur_txn)
            .or_else(|_| get_registration_info_from_txn_id(&reg_v_stmt, &cur_txn))?;

        // Handle the case where no registration is found for this txn_id
        let (prv_txn_id, mut slot_no, mut cat_id, mut txn_idx) = match registration_info {
            Some(info) => info,
            None => {
                // This txn_id exists in stake address table but not in registration table
                // This SHOULD NOT happen
                // Skip to next transaction
                continue;
            },
        };

        // This mean the registration in this transaction id IS NOT a root.
        // This check can be omitted, but is here just for readability.
        // Search persistent first then volatile
        if prv_txn_id.is_some() {
            (cat_id, slot_no, txn_idx) =
                match walk_chain_back(&reg_p_stmt, &reg_v_stmt, prv_txn_id)? {
                    Some((cat_id, slot, idx)) => (cat_id, slot, idx),
                    None => {
                        // Broken chain, skip to next transaction
                        continue;
                    },
                }
        }

        if let Some(id) = cat_id {
            // Perform a check on whether the cat id is already used by other valid chain.
            if is_valid_root(
                &root_validate_p_stmt,
                &root_validate_v_stmt,
                &id,
                slot_no,
                txn_idx,
            )? {
                let chain = select_rbac_registration_chain_from_cat_id(sqlite, sqlite_in_mem, &id)?;
                // Finalize statements before returning
                DatabaseStatement::finalize_statement(root_validate_p_stmt, FUNCTION_NAME)?;
                DatabaseStatement::finalize_statement(root_validate_v_stmt, FUNCTION_NAME)?;
                DatabaseStatement::finalize_statement(reg_p_stmt, FUNCTION_NAME)?;
                DatabaseStatement::finalize_statement(reg_v_stmt, FUNCTION_NAME)?;
                return Ok(chain);
            }
        }
    }
    DatabaseStatement::finalize_statement(root_validate_p_stmt, FUNCTION_NAME)?;
    DatabaseStatement::finalize_statement(root_validate_v_stmt, FUNCTION_NAME)?;
    DatabaseStatement::finalize_statement(reg_p_stmt, FUNCTION_NAME)?;
    DatabaseStatement::finalize_statement(reg_v_stmt, FUNCTION_NAME)?;
    Ok(vec![])
}

/// Get a list of transaction IDs that contain the given stake address.
fn get_txn_ids_from_stake_addr(
    stake: &[u8],
    sqlite: &Sqlite,
    table: &str,
) -> anyhow::Result<Vec<Vec<u8>>> {
    const FUNCTION_NAME: &str = "get_txn_ids_from_stake_addr";

    // Get txn_ids for the stake_address
    let stmt = DatabaseStatement::prepare_statement(
        sqlite,
        &QueryBuilder::select_txn_id_by_stake_addr(table),
        Operation::Select,
        FUNCTION_NAME,
    )?;

    bind_parameters!(stmt, FUNCTION_NAME, stake.to_vec() => "stake_address")?;

    // List of transactions ID that contain the given stake address
    let mut txn_ids: Vec<Vec<u8>> = Vec::new();
    loop {
        match stmt.step() {
            Ok(StepResult::Row) => {
                txn_ids.push(column_as::<Vec<u8>>(&stmt, 0, FUNCTION_NAME, "txn_id")?)
            },
            Ok(StepResult::Done) => break, // No more rows
            Err(e) => {
                DatabaseStatement::finalize_statement(stmt, FUNCTION_NAME)?;
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
        }
    }
    DatabaseStatement::finalize_statement(stmt, FUNCTION_NAME)?;
    Ok(txn_ids)
}

// Get registration info by transaction id.
fn get_registration_info_from_txn_id(
    stmt: &Statement,
    txn_id: &[u8],
) -> anyhow::Result<Option<(Option<Vec<u8>>, u64, Option<String>, u16)>> {
    const FUNCTION_NAME: &str = "get_registration_info_from_txn_id";

    DatabaseStatement::reset_statement(stmt, FUNCTION_NAME)?;
    bind_parameters!(stmt, FUNCTION_NAME, txn_id.to_vec() => "txn_id")?;

    let result = match stmt.step() {
        // This should have data since txn_id is extract from `rbac_stake_address`
        Ok(StepResult::Row) => Ok(Some((
            column_as::<Option<Vec<u8>>>(stmt, 0, FUNCTION_NAME, "prv_txn_id")?,
            column_as::<u64>(stmt, 1, FUNCTION_NAME, "slot_no")?,
            column_as::<Option<String>>(stmt, 2, FUNCTION_NAME, "catalyst_id")?,
            column_as::<u16>(stmt, 3, FUNCTION_NAME, "txn_idx")?,
        ))),
        Ok(StepResult::Done) => {
            return Ok(None);
        },
        Err(e) => {
            let error = format!("Failed to step: {e}");
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
    result
}

// Construct a registration chain by walking back a chain
fn walk_chain_back(
    reg_p_stmt: &Statement,
    reg_v_stmt: &Statement,
    mut prv_txn_id: Option<Vec<u8>>,
) -> anyhow::Result<Option<(Option<String>, u64, u16)>> {
    const FUNCTION_NAME: &str = "get_registration_info_from_txn_id";

    let mut cat_id = None;
    let mut slot_no = 0;
    let mut txn_idx = 0;

    while let Some(prev) = prv_txn_id.take() {
        // Reset statements before each use
        DatabaseStatement::reset_statement(reg_p_stmt, FUNCTION_NAME)?;
        DatabaseStatement::reset_statement(reg_v_stmt, FUNCTION_NAME)?;

        // Try persistent first, then volatile
        let reg_info = get_registration_info_from_txn_id(reg_p_stmt, &prev)
            .or_else(|_| get_registration_info_from_txn_id(reg_v_stmt, &prev))?;

        match reg_info {
            Some((next_prv_txn_id, next_slot_no, next_cat_id, next_txn_idx)) => {
                prv_txn_id = next_prv_txn_id;
                slot_no = next_slot_no;
                cat_id = next_cat_id;
                txn_idx = next_txn_idx;

                if prv_txn_id.is_none() {
                    break; // reached root
                }
            },
            None => {
                // Registration not found for this txn_id - broken chain
                return Ok(None);
            },
        }
    }

    Ok(Some((cat_id, slot_no, txn_idx)))
}

/// Check whether the given registration is a valid root
fn is_valid_root(
    stmt_p: &Statement,
    stmt_v: &Statement,
    cat_id: &str,
    slot_no: u64,
    txn_idx: u16,
) -> anyhow::Result<bool> {
    const FUNCTION_NAME: &str = "is_valid_root";

    // Try persistent first
    bind_parameters!(stmt_p, FUNCTION_NAME,
        cat_id.to_string() => "catalyst_id",
        slot_no => "slot_no",
        slot_no => "slot_no",
        txn_idx => "txn_idx"
    )?;

    let persistent_valid = match stmt_p.step() {
        // Have registration, so not valid
        Ok(StepResult::Row) => false,
        Ok(StepResult::Done) => true,
        Err(e) => {
            let error = format!("Failed to step: {e}");
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
    DatabaseStatement::reset_statement(stmt_p, FUNCTION_NAME)?;

    if !persistent_valid {
        return Ok(false);
    }

    // Then check volatile if persistent passed
    bind_parameters!(stmt_v, FUNCTION_NAME,
        cat_id.to_string() => "catalyst_id",
        slot_no => "slot_no",
        slot_no => "slot_no",
        txn_idx => "txn_idx"
    )?;

    let volatile_valid = match stmt_v.step() {
        Ok(StepResult::Row) => false,
        Ok(StepResult::Done) => true,
        Err(e) => {
            let error = format!("Failed to step: {e}");
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
    DatabaseStatement::reset_statement(stmt_v, FUNCTION_NAME)?;

    Ok(volatile_valid)
}
