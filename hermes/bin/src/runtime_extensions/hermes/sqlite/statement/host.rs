//! `SQLite` statement host implementation for WASM runtime.

use super::{super::state::resource_manager, core};
use crate::{
    runtime_context::HermesRuntimeContext,
    runtime_extensions::bindings::hermes::sqlite::api::{
        Errno, HostStatement, Statement, StepResult, Value,
    },
};

impl HostStatement for HermesRuntimeContext {
    /// Stores application data into parameters of the original SQL.
    ///
    /// ## Parameters
    ///
    /// - `index`: The index of the SQL parameter to be set.
    /// - `value`: The value to bind to the parameter.
    fn bind(
        &mut self,
        resource: wasmtime::component::Resource<Statement>,
        index: u32,
        value: Value,
    ) -> wasmtime::Result<Result<(), Errno>> {
        resource_manager::get_statement_app_state(self.app_name(), |app_state| {
            let app_state = app_state.ok_or_else(|| {
                wasmtime::Error::msg("Application not found for statement resource")
            })?;
            let stmt_ptr = app_state.get_object(&resource)?;
            let index = i32::try_from(index).map_err(|_| Errno::ConvertingNumeric)?;
            Ok(core::bind(*stmt_ptr as *mut _, index, value))
        })
    }

    /// Advances a statement to the next result row or to completion.
    ///
    /// After a prepared statement has been prepared, this function must be called one or
    /// more times to evaluate the statement.
    ///
    /// ## Returns
    ///
    /// A `step-result` indicating the status of the step.
    fn step(
        &mut self,
        resource: wasmtime::component::Resource<Statement>,
    ) -> wasmtime::Result<Result<StepResult, Errno>> {
        resource_manager::get_statement_app_state(self.app_name(), |app_state| {
            let app_state = app_state.ok_or_else(|| {
                wasmtime::Error::msg("Application not found for statement resource")
            })?;
            let stmt_ptr = app_state.get_object(&resource)?;
            Ok(core::step(*stmt_ptr as *mut _))
        })
    }

    /// Returns information about a single column of the current result row of a query.
    ///
    /// If the SQL statement does not currently point to a valid row, or if the column
    /// index is out of range, the result is undefined.
    ///
    /// ## Parameters
    ///
    /// - `index`: The index of the column for which information should be returned. The
    ///   leftmost column of the result set has the index 0.
    ///
    /// ## Returns
    ///
    /// The value of a result column in a specific data format.
    fn column(
        &mut self,
        resource: wasmtime::component::Resource<Statement>,
        index: u32,
    ) -> wasmtime::Result<Result<Value, Errno>> {
        resource_manager::get_statement_app_state(self.app_name(), |app_state| {
            let app_state = app_state.ok_or_else(|| {
                wasmtime::Error::msg("Application not found for statement resource")
            })?;
            let stmt_ptr = app_state.get_object(&resource)?;
            let index = i32::try_from(index).map_err(|_| Errno::ConvertingNumeric)?;
            Ok(core::column(*stmt_ptr as *mut _, index))
        })
    }

    /// Reset a prepared statement object back to its initial state, ready to be
    /// re-executed.
    ///
    /// This function clears all previous bindings, resets the statement to the beginning,
    /// and prepares it for another execution. This must be called before reusing a
    /// statement with new parameter bindings.
    fn reset(
        &mut self,
        resource: wasmtime::component::Resource<Statement>,
    ) -> wasmtime::Result<Result<(), Errno>> {
        resource_manager::get_statement_app_state(self.app_name(), |app_state| {
            let app_state = app_state.ok_or_else(|| {
                wasmtime::Error::msg("Application not found for statement resource")
            })?;
            let stmt_ptr = app_state.get_object(&resource)?;
            Ok(core::reset(*stmt_ptr as *mut _))
        })
    }

    /// Destroys a prepared statement object. If the most recent evaluation of the
    /// statement encountered no errors or if the statement is never been evaluated,
    /// then the function results without errors. If the most recent evaluation of
    /// statement failed, then the function results the appropriate error code.
    ///
    /// The application must finalize every prepared statement in order to avoid resource
    /// leaks. It is a grievous error for the application to try to use a prepared
    /// statement after it has been finalized. Any use of a prepared statement after
    /// it has been finalized can result in undefined and undesirable behavior such as
    /// segfaults and heap corruption.
    fn finalize(
        &mut self,
        resource: wasmtime::component::Resource<Statement>,
    ) -> wasmtime::Result<Result<(), Errno>> {
        resource_manager::get_statement_app_state(self.app_name(), |app_state| {
            let app_state = app_state.ok_or_else(|| {
                wasmtime::Error::msg("Application not found for statement resource")
            })?;
            let stmt_ptr = app_state.delete_resource(&resource)?;
            Ok(core::finalize(stmt_ptr as *mut _))
        })
    }

    fn drop(
        &mut self,
        resource: wasmtime::component::Resource<Statement>,
    ) -> wasmtime::Result<()> {
        resource_manager::get_statement_app_state(self.app_name(), |app_state| {
            if let Some(app_state) = app_state {
                if let Ok(stmt_ptr) = app_state.delete_resource(&resource) {
                    let _ = core::finalize(stmt_ptr as *mut _);
                }
            }
            Ok(())
        })
    }
}
