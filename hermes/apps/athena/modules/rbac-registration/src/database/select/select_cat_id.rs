//! Select catalyst ID from the `rbac_registration` table.

use crate::{
    database::{bind_with_log, select::column_as},
    hermes::hermes::{
        cardano::api::CardanoNetwork,
        sqlite::api::{Sqlite, StepResult, Value},
    },
    rbac::build_rbac_chain::RbacChainInfo,
    utils::log::log_error,
};

/// The root of the registration chain is the registration with matching `catalyst_id` where
/// no `prv_txn_id`, no `problem_report` - valid, least `slot_no`, and least `txn_idx`.
const RBAC_SELECT_ROOT_REGISTRATION_CHAIN_BY_CAT_ID: &str = r#"
    SELECT txn_id, slot_no, txn_idx
        FROM rbac_registration
        WHERE prv_txn_id IS NULL
        AND problem_report IS NULL
        AND catalyst_id = ?
        ORDER BY slot_no ASC, txn_idx ASC
        LIMIT 1
"#;

/// Find the next registration in the chain.
/// For example, requesting for `catalyst_id` = `cat_id` resulting in registration_A with `txn_id` = `abc`.
/// The next child of this registration is a registration that has `prv_txn_id` = `abc`, problem_report is null,
/// least `slot_no` and least `txn_idx`. This process repeat until no more child is found.
const RBAC_SELECT_CHILD_REGISTRATION_CHAIN_FROM_CAT_ID: &str = r#"
    SELECT txn_id, slot_no, txn_idx FROM rbac_registration 
        WHERE prv_txn_id = ?
        AND problem_report IS NULL 
        ORDER BY slot_no ASC, txn_idx ASC 
        LIMIT 1
"#;

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
    cat_id: &str,
    network: CardanoNetwork,
) -> anyhow::Result<Vec<RbacChainInfo>> {
    const FUNCTION_NAME: &str = "select_rbac_registration_chain_from_cat_id";

    // --- Find the root ---
    let (mut txn_id, mut chain) = if let Some(r) = extract_root(sqlite, cat_id)? {
        r
    } else {
        return Ok(vec![]);
    };

    // --- Find children ---
    let stmt = sqlite
        .prepare(RBAC_SELECT_CHILD_REGISTRATION_CHAIN_FROM_CAT_ID)
        .map_err(|e| {
            let err =
                format!("Failed to prepare RBAC_SELECT_CHILD_REGISTRATION_CHAIN_FROM_CAT_ID: {e}");
            log_error(
                file!(),
                FUNCTION_NAME,
                "hermes::sqlite::api::prepare",
                &err,
                None,
            );
            anyhow::anyhow!(err)
        })?;
    // From root, step to the next child recursively
    loop {
        // Reset before rebinding
        stmt.reset().map_err(|e| {
            log_error(
                file!(),
                FUNCTION_NAME,
                "hermes::sqlite::api::reset",
                &format!("Failed to reset rbac child stmt: {e}"),
                None,
            );
            anyhow::anyhow!("Failed to reset rbac child stmt: {e}")
        });

        bind_with_log(&stmt, FUNCTION_NAME, 1, &txn_id.into(), "txn_id")?;

        match stmt.step() {
            Ok(StepResult::Row) => {
                // Need this for looping, so no need to convert to rust type
                let next_txn_id = stmt.column(0)?;
                let slot_no = column_as::<u64>(&stmt, 1, FUNCTION_NAME, "slot_no")?;
                let txn_idx = column_as::<u16>(&stmt, 2, FUNCTION_NAME, "txn_idx")?;

                txn_id = next_txn_id;
                chain.push(RbacChainInfo { slot_no, txn_idx });
            },
            // At the end of the chain
            Ok(StepResult::Done) => break,
            Err(e) => {
                log_error(
                    file!(),
                    FUNCTION_NAME,
                    "hermes::sqlite::api::step",
                    &format!("Failed to step: {e}"),
                    None,
                );
                return Err(anyhow::anyhow!(e));
            },
        }
    }

    stmt.finalize()?;
    Ok(chain)
}

/// Extract the root registration.
fn extract_root(
    sqlite: &Sqlite,
    cat_id: &str,
) -> anyhow::Result<Option<(Value, Vec<RbacChainInfo>)>> {
    const FUNCTION_NAME: &str = "extract_root";

    let stmt = sqlite
        .prepare(RBAC_SELECT_ROOT_REGISTRATION_CHAIN_BY_CAT_ID)
        .map_err(|e| {
            let err =
                format!("Failed to prepare rbac root statement: {e}");
            log_error(
                file!(),
                FUNCTION_NAME,
                "hermes::sqlite::api::prepare",
                &err,
                None,
            );
            anyhow::anyhow!(err)
        })?;

    bind_with_log(
        &stmt,
        FUNCTION_NAME,
        1,
        &cat_id.to_string().into(),
        "catalyst_id",
    )?;

    let result = match stmt.step() {
        Ok(StepResult::Row) => {
            let txn_id = stmt.column(0)?;
            let slot_no = column_as::<u64>(&stmt, 1, FUNCTION_NAME, "slot_no")?;
            let txn_idx = column_as::<u16>(&stmt, 2, FUNCTION_NAME, "txn_idx")?;

            Ok(Some((
                txn_id.clone(),
                vec![RbacChainInfo { slot_no, txn_idx }],
            )))
        },
        // No row = no root
        Ok(StepResult::Done) => Ok(None),
        Err(e) => {
            stmt.finalize()?;
            log_error(
                file!(),
                FUNCTION_NAME,
                "hermes::sqlite::api::step",
                &format!("Failed to step: {e}"),
                None,
            );
            Err(anyhow::anyhow!(e))
        },
    };

    stmt.finalize()?;
    result
}
