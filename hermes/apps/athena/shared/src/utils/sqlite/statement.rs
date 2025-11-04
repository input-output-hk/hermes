//! `SQLite` statement utilities.

use crate::{
    bindings::hermes::sqlite::api::{Sqlite, Statement, Value},
    sqlite_bind_parameters,
    utils::{log::log_error, sqlite::operation::Operation},
};

/// Database statement utilities.
pub struct DatabaseStatement;

impl DatabaseStatement {
    /// Execute a statement.
    ///
    /// # Errors
    ///
    /// Returns an error if `SQLite` returns one during execution.
    pub fn execute_statement(
        sqlite: &Sqlite,
        query: &str,
        operation: Operation,
        func_name: &str,
    ) -> anyhow::Result<()> {
        if let Err(e) = sqlite.execute(query) {
            let error = format!("Failed to execute {operation}: {e}");
            log_error(
                file!(),
                func_name,
                "hermes::sqlite::api::execute",
                &error,
                None,
            );
            anyhow::bail!(error);
        }
        Ok(())
    }

    /// Prepare statement.
    ///
    /// # Errors
    ///
    /// Returns an error if `SQLite` returns one during statement preparation.
    pub fn prepare_statement(
        sqlite: &Sqlite,
        query: &str,
        operation: Operation,
        func_name: &str,
    ) -> anyhow::Result<Statement> {
        match sqlite.prepare(query) {
            Ok(stmt) => Ok(stmt),
            Err(e) => {
                let error = format!("Failed to prepare {operation} statement: {e}");
                log_error(
                    file!(),
                    func_name,
                    "hermes::sqlite::api::prepare",
                    &error,
                    None,
                );
                anyhow::bail!(error);
            },
        }
    }

    /// Reset a statement.
    ///
    /// # Errors
    ///
    /// Returns an error if `SQLite` returns one during reset.
    pub fn reset_statement(
        stmt: &Statement,
        func_name: &str,
    ) -> anyhow::Result<()> {
        if let Err(e) = stmt.reset() {
            let error = format!("Failed to reset {e}");
            log_error(
                file!(),
                func_name,
                "hermes::sqlite::api::reset",
                &error,
                None,
            );
            anyhow::bail!(error);
        }
        Ok(())
    }

    /// Finalize a statement.
    ///
    /// # Errors
    ///
    /// Returns an error if `SQLite` returns one during finalize.
    pub fn finalize_statement(
        stmt: Statement,
        func_name: &str,
    ) -> anyhow::Result<()> {
        if let Err(e) = Statement::finalize(stmt) {
            let error = format!("Failed to finalize {e}");
            log_error(
                file!(),
                func_name,
                "hermes::sqlite::api::finalize",
                &error,
                None,
            );
            anyhow::bail!(error);
        }
        Ok(())
    }

    /// Bind -> step -> reset a prepared statement.
    ///
    /// # Errors
    ///
    /// Returns an error if binding, stepping, or reset fails.
    pub fn bind_step_reset_statement<F>(
        stmt: &Statement,
        bind_fn: F,
        func_name: &str,
    ) -> anyhow::Result<()>
    where
        F: FnOnce(&Statement) -> anyhow::Result<()>,
    {
        bind_fn(stmt)?;
        if let Err(e) = stmt.step() {
            let error = format!("Failed to step: {e}");
            crate::utils::log::log_error(
                file!(),
                func_name,
                "hermes::sqlite::api::step",
                &error,
                None,
            );
            anyhow::bail!(error);
        }
        Self::reset_statement(stmt, func_name)?;

        Ok(())
    }

    /// Bind `slot_no` to a prepared statement.
    /// This is commonly used to bind a slot to a prepared statement.
    ///
    /// # Errors
    ///
    /// Returns an error if `SQLite` returns one during bind or if the slot
    /// number cannot be converted.
    pub fn bind_slot(
        stmt: &Statement,
        slot_no: u64,
        func_name: &str,
    ) -> anyhow::Result<()> {
        fn bind(
            stmt: &Statement,
            slot_no: u64,
            func_name: &str,
        ) -> anyhow::Result<()> {
            let slot: Value = match slot_no.try_into() {
                Ok(s) => s,
                Err(e) => {
                    let error = format!("Failed to convert slot: {e}");
                    log_error(file!(), func_name, "slot.try_into()", &error, None);
                    anyhow::bail!(error);
                },
            };
            sqlite_bind_parameters!(stmt, func_name, slot => "slot_no")?;
            Ok(())
        }

        Self::bind_step_reset_statement(stmt, |stmt| bind(stmt, slot_no, func_name), func_name)?;
        Ok(())
    }
}

/// Convert an `SQLite` column value to a Rust type.
///
/// # Errors
///
/// Returns an error if `SQLite` returns one during the column call
/// or if fetched value could not be casted to provided generic type.
pub fn column_as<T>(
    stmt: &Statement,
    index: u32,
    func_name: &str,
    field_name: &str,
) -> anyhow::Result<T>
where
    T: TryFrom<Value, Error = anyhow::Error>,
{
    let value = stmt.column(index)?;
    T::try_from(value).map_err(|e| {
        log_error(
            file!(),
            func_name,
            "column_as",
            &format!(
                "Failed to convert column {} to {}: {}",
                field_name,
                std::any::type_name::<T>(),
                e
            ),
            None,
        );
        e
    })
}
