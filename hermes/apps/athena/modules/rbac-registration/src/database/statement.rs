//! SQLite Statement implementation.

use crate::{
    bind_parameters,
    database::operation::Operation,
    hermes::sqlite::api::{Sqlite, Statement, Value},
    utils::log::log_error,
};

/// Database statement.
pub struct DatabaseStatement;

impl DatabaseStatement {
    /// Execute a statement.
    pub(crate) fn execute_statement(
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
    pub(crate) fn prepare_statement(
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
    pub(crate) fn reset_statement(
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
    pub(crate) fn finalize_statement(
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
    pub(crate) fn bind_step_reset_statement<F>(
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

    /// Bind slot to prepared statement.
    /// This is commonly used to bind slot to a prepared statement.
    pub(crate) fn bind_slot(
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
            bind_parameters!(stmt, func_name, slot => "slot_no")?;
            Ok(())
        }

        Self::bind_step_reset_statement(stmt, |stmt| bind(stmt, slot_no, func_name), func_name)?;
        Ok(())
    }
}
