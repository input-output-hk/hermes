//! Select from Catalyst ID.

use shared::{
    bindings::hermes::sqlite::api::{Sqlite, Statement, StepResult, Value},
    sqlite_bind_parameters,
    utils::{
        log::log_error,
        sqlite::{
            operation::Operation,
            statement::{DatabaseStatement, column_as},
        },
    },
};

use crate::{
    database::{
        RBAC_REGISTRATION_PERSISTENT_TABLE_NAME, RBAC_REGISTRATION_VOLATILE_TABLE_NAME,
        query_builder::QueryBuilder, select::TableSource,
    },
    rbac::{rbac_chain_metadata::RbacChainMetadata, registration_location::RegistrationLocation},
};

/// Registration chain from a catalyst ID.
///
/// Selects a root registration and all its children, returning the full chain given a
/// catalyst ID. If no root registration is found for the given `cat_id`, returns an empty
/// list.
///
/// Root registration:
///
/// The root registration is the registration with matching `catalyst_id` where
/// no `prv_txn_id`, no `problem_report` - valid, least `slot_no`, and least `txn_idx`.
/// In other words, the earliest valid registration containing the given `catalyst_id` is
/// considered the root. Note that the Catalyst ID is derive from the subject public key
/// or Role 0 registration. If the registration chain contains multiple Catalyst IDs
/// (multiple Role 0 subject public keys), the first catalyst ID in the chain is used.
///
/// For example, a chain with the first registration having `catalyst_id_a` and the second
/// registration having `catalyst_id_b`, When requesting for `catalyst_id_b` WILL NOT
/// result in the same chain as requesting for `catalyst_id_a`.
///
/// In addition, if there is an attempt to creating a new chain with `catalyst_id_b`, the
/// chain will be invalid since **IT IS NOT ALLOWED TO USE THE PUBLIC KEYS OF AN EXISTING
/// VALID CHAIN**.
///
///
/// Child registration:
///
/// The child registration is determined by having a `prv_txn_id` pointing back to a
/// parent. In other words, the child registration is the registration with matching
/// `prv_txn_id` where no `problem_report` - valid, least `slot_no`, and least `txn_idx`.
///
/// An update that causes the link to break is considered invalid.
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
/// * `Ok(Vec<RegistrationLocation>, RbacChainMetadata))` – The registration chain related
///   data associated with the given catalyst ID. If the vector is empty, no chain is
///   found.
/// * `Err(anyhow::Error)` – If any error occurs.
pub(crate) fn select_rbac_registration_chain_from_cat_id(
    persistent: &Sqlite,
    volatile: &Sqlite,
    cat_id: &str,
) -> anyhow::Result<(Vec<RegistrationLocation>, RbacChainMetadata)> {
    const FUNCTION_NAME: &str = "select_rbac_registration_chain_from_cat_id";

    let mut metadata = RbacChainMetadata::default();

    // --- Find the root ---
    let Some((mut txn_id, mut chain, root_source)) =
        extract_root(persistent, cat_id, RBAC_REGISTRATION_PERSISTENT_TABLE_NAME)?.or_else(|| {
            extract_root(volatile, cat_id, RBAC_REGISTRATION_VOLATILE_TABLE_NAME).ok()?
        })
    else {
        return Ok((vec![], metadata));
    };
    // Update tracking variable based on the root source
    match root_source {
        TableSource::Persistent => {
            metadata.last_persistent_txn = Some(txn_id.clone().try_into()?);
            // This should not fail
            metadata.last_persistent_slot = chain
                .first()
                .ok_or_else(|| anyhow::anyhow!("Chain is empty when extracting slot_no"))?
                .slot_no
                .into();
        },
        TableSource::Volatile => {
            metadata.last_volatile_txn = Some(txn_id.clone().try_into()?);
        },
    }

    // --- Find children ---
    let p_stmt = DatabaseStatement::prepare_statement(
        persistent,
        &QueryBuilder::select_child_reg_from_parent(RBAC_REGISTRATION_PERSISTENT_TABLE_NAME),
        Operation::Select,
        FUNCTION_NAME,
    )?;

    let v_stmt = DatabaseStatement::prepare_statement(
        volatile,
        &QueryBuilder::select_child_reg_from_parent(RBAC_REGISTRATION_VOLATILE_TABLE_NAME),
        Operation::Select,
        FUNCTION_NAME,
    )?;
    let result: anyhow::Result<Vec<RegistrationLocation>> = (|| {
        loop {
            // Persistent first
            if let Some((next_txn_id, slot_no, txn_idx)) =
                extract_child(&p_stmt, RBAC_REGISTRATION_PERSISTENT_TABLE_NAME, &txn_id)?
            {
                txn_id = next_txn_id;
                chain.push(RegistrationLocation { slot_no, txn_idx });

                metadata.last_persistent_txn = Some(txn_id.clone().try_into()?);
                metadata.last_persistent_slot = slot_no.into();
                continue;
            }

            // Then volatile
            match extract_child(&v_stmt, RBAC_REGISTRATION_VOLATILE_TABLE_NAME, &txn_id)? {
                Some((next_txn_id, slot_no, txn_idx)) => {
                    txn_id = next_txn_id;
                    chain.push(RegistrationLocation { slot_no, txn_idx });

                    metadata.last_volatile_txn = Some(txn_id.clone().try_into()?);
                },
                None => break,
            }
        }
        Ok(chain)
    })();

    let _unused = DatabaseStatement::finalize_statement(p_stmt, FUNCTION_NAME);
    let _unused = DatabaseStatement::finalize_statement(v_stmt, FUNCTION_NAME);

    result.map(|chain| (chain, metadata))
}

/// Extract the root registration.
fn extract_root(
    sqlite: &Sqlite,
    cat_id: &str,
    table: &str,
) -> anyhow::Result<Option<(Value, Vec<RegistrationLocation>, TableSource)>> {
    const FUNCTION_NAME: &str = "extract_root";

    let stmt = DatabaseStatement::prepare_statement(
        sqlite,
        &QueryBuilder::select_root_reg_by_cat_id(table),
        Operation::Select,
        FUNCTION_NAME,
    )?;
    sqlite_bind_parameters!(stmt, FUNCTION_NAME, cat_id.to_string() => "catalyst_id")?;

    // The first valid root registration is chosen
    let result = (|| {
        match stmt.step() {
            Ok(StepResult::Row) => {
                let txn_id = stmt.column(0)?;
                let slot_no = column_as::<u64>(&stmt, 1, FUNCTION_NAME, "slot_no")?;
                let txn_idx = column_as::<u16>(&stmt, 2, FUNCTION_NAME, "txn_idx")?;
                // Should be able to track which table the root came from
                let source = if table == RBAC_REGISTRATION_PERSISTENT_TABLE_NAME {
                    TableSource::Persistent
                } else {
                    TableSource::Volatile
                };
                Ok(Some((
                    txn_id.clone(),
                    vec![RegistrationLocation { slot_no, txn_idx }],
                    source,
                )))
            },
            Ok(StepResult::Done) => Ok(None),
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
        }
    })();
    DatabaseStatement::finalize_statement(stmt, FUNCTION_NAME)?;
    result
}

/// Extract the child registration.
fn extract_child(
    stmt: &Statement,
    table: &str,
    txn_id: &Value,
) -> anyhow::Result<Option<(Value, u64, u16)>> {
    const FUNCTION_NAME: &str = "extract_child";

    // Reset first to ensure the statement is in a clean state
    DatabaseStatement::reset_statement(stmt, FUNCTION_NAME)?;
    sqlite_bind_parameters!(stmt, FUNCTION_NAME, txn_id.clone() => "txn_id")?;
    let result = match stmt.step() {
        Ok(StepResult::Row) => {
            let next_txn_id = stmt.column(0)?;
            let slot_no = column_as::<u64>(stmt, 1, FUNCTION_NAME, "slot_no")?;
            let txn_idx = column_as::<u16>(stmt, 2, FUNCTION_NAME, "txn_idx")?;
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
