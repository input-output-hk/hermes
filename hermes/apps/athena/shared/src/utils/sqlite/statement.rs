//! SQLite Statement implementation.

use crate::{
    bindings::hermes::sqlite::api::{Sqlite, Statement, StepResult, Value},
    sqlite_bind_parameters,
    utils::{log::log_error, sqlite::operation::Operation},
};

/// Database statement.
pub struct DatabaseStatement;

impl DatabaseStatement {
    /// Execute a statement.
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

    /// Bind slot to prepared statement.
    /// This is commonly used to bind slot to a prepared statement.
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

    /// Selects all rows returned by statement.
    pub fn select_all<T>(
        stmt: &Statement,
        func_name: &str,
    ) -> anyhow::Result<Vec<T>>
    where
        T: RowAs,
    {
        let mut rows = vec![];
        loop {
            match stmt.step() {
                Ok(StepResult::Row) => {
                    let row = T::try_from_stmt(stmt, "select_all")?;
                    rows.push(row);
                },
                Ok(StepResult::Done) => break,
                Err(error) => {
                    Self::reset_statement(stmt, "select_all")?;
                    log_error(
                        file!(),
                        func_name,
                        "select_all",
                        &format!("Failed to make step: {}", error),
                        None,
                    );
                    anyhow::bail!(error);
                },
            }
        }

        Ok(rows)
    }
}

/// Convert a SQLite column value to a Rust type.
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

/// Trait for parsing SQLite row into type
///
/// Note: Already implemented for tuples of
/// size up to 12 (but each item has to implement `TryFrom<Value, Error = anyhow::Error>`)
pub trait RowAs: Sized {
    /// Attempts to construct the type from the given statement,
    /// starting at the specified column index.
    fn try_from_stmt(
        stmt: &Statement,
        func_name: &str,
    ) -> anyhow::Result<Self>;
}

macro_rules! impl_row_as_for_tuple {
    // Takes both the type $T and the static index token $idx
    ( $( $T:ident $idx:tt ),+ ) => {
        impl<$($T),+> RowAs for ($($T,)+)
        where
            $($T: TryFrom<Value, Error = anyhow::Error>,)+
        {
            fn try_from_stmt(
                stmt: &Statement,
                func_name: &str,
            ) -> anyhow::Result<Self> {
                Ok((
                    $(
                        column_as::<$T>(
                            stmt,
                            $idx,
                            func_name,
                            &format!("column_{}", $idx),
                        )?,
                    )+
                ))
            }
        }
    };
}

impl_row_as_for_tuple! { T1 0 }
impl_row_as_for_tuple! { T1 0, T2 1 }
impl_row_as_for_tuple! { T1 0, T2 1, T3 2 }
impl_row_as_for_tuple! { T1 0, T2 1, T3 2, T4 3 }
impl_row_as_for_tuple! { T1 0, T2 1, T3 2, T4 3, T5 4 }
impl_row_as_for_tuple! { T1 0, T2 1, T3 2, T4 3, T5 4, T6 5 }
impl_row_as_for_tuple! { T1 0, T2 1, T3 2, T4 3, T5 4, T6 5, T7 6 }
impl_row_as_for_tuple! { T1 0, T2 1, T3 2, T4 3, T5 4, T6 5, T7 6, T8 7 }
impl_row_as_for_tuple! { T1 0, T2 1, T3 2, T4 3, T5 4, T6 5, T7 6, T8 7, T9 8 }
impl_row_as_for_tuple! { T1 0, T2 1, T3 2, T4 3, T5 4, T6 5, T7 6, T8 7, T9 8, T10 9 }
impl_row_as_for_tuple! { T1 0, T2 1, T3 2, T4 3, T5 4, T6 5, T7 6, T8 7, T9 8, T10 9, T11 10 }
impl_row_as_for_tuple! { T1 0, T2 1, T3 2, T4 3, T5 4, T6 5, T7 6, T8 7, T9 8, T10 9, T11 10, T12 11 }
