//! Select stake address from the rbac_stake_address table.

use cardano_blockchain_types::StakeAddress;

use crate::{
    database::{
        bind_with_log,
        select::{column_as, select_cat_id::select_rbac_registration_chain_from_cat_id},
    },
    hermes::hermes::{
        cardano::api::CardanoNetwork,
        sqlite::api::{Sqlite, Statement, StepResult},
    },
    rbac::build_rbac_chain::RbacChainInfo,
    utils::log::log_error,
};

/// Select transaction ID that the stake address belongs to.
/// Stake address can be replaced by the newest valid registration chain,
/// so order by latest slot and latest txn_idx.
///
/// The data from `rbac_stake_address` itself cannot indicate that the stake address
/// is in a valid chain or not. Hence all associated data need to be checked.
const RBAC_STAKE_ADDR_SELECT_TXN_ID_BY_STAKE_ADDR: &str = r#"
    SELECT txn_id
        FROM rbac_stake_address
        WHERE stake_address = ?
        ORDER BY slot_no DESC, txn_idx DESC
"#;

/// Get the registration information from the transaction ID.
/// The earliest (lowest `slot_no`, then lowest `txn_idx`) registration
/// is considered the canonical/valid registration if multiple exist.
const RBAC_SELECT_REG_BY_TXN_ID_ORDER_SLOT_TXN_IDX: &str = r#"
    SELECT prv_txn_id, slot_no, catalyst_id, txn_idx FROM rbac_registration
        WHERE txn_id = ?
        AND problem_report IS NULL
        ORDER BY slot_no ASC, txn_idx ASC
"#;

/// Find all root registration that has this given catalyst ID.
/// It should come before the given slot_id and txn_idx.
/// eg. The input is slot_no = 20, txn_idx = 2.
/// It will give these registrations:
/// slot_no = 20, txn_id = 1
/// slot_no = 9 txn_id = 2
/// slot_no = txn_id = 0
/// This is use to validate that the given root is valid or not.
/// Note that the valid root should have the least `slot_no` and least `txn_idx` with no problem report.
const RBAC_SELECT_ROOT_REG_BY_CAT_ID_LESS_THAN_SLOT_TXN_IDX: &str = r#"
    SELECT txn_id FROM rbac_registration
        WHERE prv_txn_id IS NULL
        AND problem_report IS NULL
        AND catalyst_id = ?
        AND (
            slot_no < ? OR (slot_no = ? AND txn_idx < ?)
        )  
"#;

/// Registration chain from a stake address:
///
/// 1. Start from the newest registration.
/// - The rule is the newest `stake_address` that is in a valid chain, will take over that `stake_address` in the older chain.
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
/// The there will be 2 cases
///
/// a. Root registration (prv_txn_id IS NULL):
/// - A valid root must have a non-null catalyst_id, no problem_report, and no prv_txn_id.
/// - Then need to validate by checking whether this catalyst_id was already used
///   in an earlier root (slot_no < slot_no_root, or same slot with smaller txn_idx).
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
/// b. Non-root registration (prv_txn_id IS NOT NULL):
/// - Follow the chain backwards (txn_id -> prv_txn_id -> …) until reaching a root (prv_txn_id IS NULL).
/// - Validate that root using the same rule as (a).
/// - If root is valid -> stake_address belongs to this chain.
/// - Otherwise -> continue with next `txn_id`.
///
/// 3. If no valid root/chain is found after checking all candidates, then `stake_address` does not belong to any valid registration.
pub(crate) fn select_rbac_registration_chain_from_stake_addr(
    sqlite: &Sqlite,
    stake_addr: StakeAddress,
    network: CardanoNetwork,
) -> anyhow::Result<Vec<RbacChainInfo>> {
    const FUNCTION_NAME: &str = "select_rbac_registration_chain_from_stake_addr";

    // Convert the given stake address to Vec<u8> which will be use in the query
    let stake: Vec<u8> = stake_addr.try_into().map_err(|e| {
        log_error(
            file!(),
            FUNCTION_NAME,
            "stake_addr.try_into",
            "Failed to convert stake address to Vec<u8>",
            None,
        );
        anyhow::anyhow!("Failed to convert stake address StakeAddress: {e:?}")
    })?;
    // List of transaction IDs that contain the given stake address
    let txn_ids = get_txn_ids_from_stake_addr(stake, sqlite)?;

    let mut stmt_reg = sqlite
        .prepare(RBAC_SELECT_REG_BY_TXN_ID_ORDER_SLOT_TXN_IDX)
        .map_err(|e| {
            let err =
                format!("Failed to prepare RBAC_SELECT_REG_BY_TXN_ID_ORDER_SLOT_TXN_IDX: {e}");
            log_error(
                file!(),
                FUNCTION_NAME,
                "hermes::sqlite::api::prepare",
                &err,
                None,
            );
            anyhow::anyhow!(err)
        })?;
    let mut stmt_validate_root = sqlite
        .prepare(RBAC_SELECT_ROOT_REG_BY_CAT_ID_LESS_THAN_SLOT_TXN_IDX)
        .map_err(|e| {
            let err = format!(
                "Failed to prepare RBAC_SELECT_ROOT_REG_BY_CAT_ID_LESS_THAN_SLOT_TXN_IDX: {e}"
            );
            log_error(
                file!(),
                FUNCTION_NAME,
                "hermes::sqlite::api::prepare",
                &err,
                None,
            );
            anyhow::anyhow!(err)
        })?;

    for mut cur_txn in txn_ids {
        let (mut prv_txn_id, mut slot_no, mut cat_id, mut txn_idx) =
            get_registration_info_from_txn_id(&stmt_reg, cur_txn, FUNCTION_NAME)?;

        // This mean the registration in this transaction id IS NOT a root.
        // This check can be omitted, but is here just for readability.
        if prv_txn_id.is_some() {
            (cat_id, slot_no, txn_idx) =
                match walk_chain_back(&stmt_reg, prv_txn_id, FUNCTION_NAME)? {
                    Some((cat_id, slot, idx)) => (cat_id, slot, idx),
                    None => {
                        // Broken chain, skip to next transaction
                        stmt_reg.reset()?;
                        stmt_validate_root.reset()?;
                        continue;
                    },
                }
        }

        if let Some(id) = cat_id {
            // Perform a check on whether the cat id is already used by other valid chain.
            if is_valid_root(&stmt_validate_root, &id, slot_no, txn_idx)? {
                let chain = select_rbac_registration_chain_from_cat_id(sqlite, &id, network)?;
                stmt_reg.finalize()?;
                stmt_validate_root.finalize()?;
                return Ok(chain);
            }
        }
        stmt_reg.reset()?;
        stmt_validate_root.reset()?;
    }
    stmt_reg.reset()?;
    stmt_validate_root.reset()?;
    Ok(vec![])
}

/// Get a list of transaction IDs that contain the given stake address.
fn get_txn_ids_from_stake_addr(
    stake: Vec<u8>,
    sqlite: &Sqlite,
) -> anyhow::Result<Vec<Vec<u8>>> {
    const FUNCTION_NAME: &str = "get_txn_ids_from_stake_addr";

    // --- Get txn_ids for the stake_address, newest first ---
    let mut stmt = sqlite
        .prepare(RBAC_STAKE_ADDR_SELECT_TXN_ID_BY_STAKE_ADDR)
        .map_err(|e| {
            let err = format!("Failed to prepare RBAC_STAKE_ADDR_SELECT_TXN_ID_BY_STAKE_ADDR: {e}");
            log_error(
                file!(),
                FUNCTION_NAME,
                "hermes::sqlite::api::prepare",
                &err,
                None,
            );
            anyhow::anyhow!(err)
        })?;

    bind_with_log(&stmt, FUNCTION_NAME, 1, &stake.into(), "stake_address")?;

    // List of transactions ID that contain the given stake address
    let mut txn_ids: Vec<Vec<u8>> = Vec::new();
    loop {
        match stmt.step() {
            Ok(StepResult::Row) => {
                txn_ids.push(column_as::<Vec<u8>>(&stmt, 0, FUNCTION_NAME, "txn_id")?)
            },
            Ok(StepResult::Done) => break,
            Err(e) => {
                stmt.finalize()?;
                log_error(
                    file!(),
                    FUNCTION_NAME,
                    "step",
                    &format!("Failed to step: {e}"),
                    None,
                );
                return Err(anyhow::anyhow!(e));
            },
        }
    }
    stmt.finalize()?;
    Ok(txn_ids)
}

// Get registration info by transaction id.
fn get_registration_info_from_txn_id(
    stmt: &Statement,
    txn_id: Vec<u8>,
    function_name: &str,
) -> anyhow::Result<(Option<Vec<u8>>, u64, Option<String>, u16)> {
    bind_with_log(stmt, function_name, 1, &txn_id.clone().into(), "txn_id")?;

    let result = match stmt.step() {
        // This should have data since txn_id is extract from `rbac_stake_address`
        Ok(StepResult::Row) => Ok((
            column_as::<Option<Vec<u8>>>(stmt, 0, function_name, "prv_txn_id")?,
            column_as::<u64>(stmt, 1, function_name, "slot_no")?,
            column_as::<Option<String>>(stmt, 2, function_name, "catalyst_id")?,
            column_as::<u16>(stmt, 3, function_name, "txn_idx")?,
        )),
        Ok(StepResult::Done) => {
            let err = format!("There should be some data given txn_id: {:?}", &txn_id);
            log_error(
                file!(),
                function_name,
                "hermes::sqlite::api::StepResult::Done",
                &err,
                None,
            );
            Err(anyhow::anyhow!(err))
        },
        Err(e) => {
            log_error(
                file!(),
                function_name,
                "hermes::sqlite::api::step",
                &format!("Failed to step: {e}"),
                None,
            );
            Err(anyhow::anyhow!(e))
        },
    };
    stmt.reset()?;
    result
}

fn is_valid_root(
    stmt: &Statement,
    cat_id: &str,
    slot_no: u64,
    txn_idx: u16,
) -> anyhow::Result<bool> {
    const FUNCTION_NAME: &str = "is_valid_root";

    bind_with_log(
        &stmt,
        FUNCTION_NAME,
        1,
        &cat_id.to_string().into(),
        "catalyst_id",
    )?;
    bind_with_log(&stmt, FUNCTION_NAME, 2, &slot_no.try_into()?, "slot_no")?;
    bind_with_log(&stmt, FUNCTION_NAME, 3, &slot_no.try_into()?, "slot_no")?;
    bind_with_log(&stmt, FUNCTION_NAME, 4, &txn_idx.into(), "txn_idx")?;

    // If there is any registration, this mean the current registration is invalid.
    let is_valid = match stmt.step() {
        Ok(StepResult::Row) => false,
        Ok(StepResult::Done) => true,
        Err(e) => {
            log_error(
                file!(),
                FUNCTION_NAME,
                "step",
                &format!("Failed to step: {e}"),
                None,
            );
            return Err(anyhow::anyhow!(e));
        },
    };

    stmt.reset()?;
    Ok(is_valid)
}

// Construct a registration chain by walking back a chain
fn walk_chain_back(
    stmt: &Statement,
    mut prv_txn_id: Option<Vec<u8>>,
    function_name: &str,
) -> anyhow::Result<Option<(Option<String>, u64, u16)>> {
    let mut cat_id = None;
    let mut slot_no = 0;
    let mut txn_idx = 0;

    // Walk back until there is no previous transaction, which mean it is at root
    // or until registration not found for a given previous transaction.
    while let Some(prev) = prv_txn_id.take() {
        bind_with_log(stmt, function_name, 1, &prev.into(), "txn_id")?;

        match stmt.step() {
            Ok(StepResult::Row) => {
                prv_txn_id = column_as::<Option<Vec<u8>>>(stmt, 0, function_name, "prv_txn_id")?;
                slot_no = column_as::<u64>(stmt, 1, function_name, "slot_no")?;
                cat_id = column_as::<Option<String>>(stmt, 2, function_name, "catalyst_id")?;
                txn_idx = column_as::<u16>(stmt, 3, function_name, "txn_idx")?;
            },
            // Broken chain
            Ok(StepResult::Done) => {
                stmt.reset()?;
                return Ok(None);
            },
            Err(e) => {
                log_error(
                    file!(),
                    function_name,
                    "hermes::sqlite::api::step",
                    &format!("Failed to step: {e}"),
                    None,
                );
                return Err(anyhow::anyhow!(e));
            },
        }
        stmt.reset()?;
    }

    Ok(Some((cat_id, slot_no, txn_idx)))
}
