// cspell: words errcode

//! `SQLite` connection object host implementation for WASM runtime.

use super::{super::state::get_db_state, core};
use crate::{
    runtime_context::HermesRuntimeContext,
    runtime_extensions::{
        bindings::hermes::sqlite::api::{Errno, ErrorInfo, HostSqlite, Sqlite, Statement},
        hermes::sqlite::state::get_statement_state,
    },
};

impl HostSqlite for HermesRuntimeContext {
    /// Closes a database connection, destructor for `sqlite3`.
    ///
    /// Ideally, applications should finalize all prepared statements associated with the
    /// `sqlite3` object prior to attempting to close the object. If the database
    /// connection is associated with unfinalized prepared statements,
    /// then the function will leave the database connection open and return the `busy`
    /// error code.
    ///
    /// If an `sqlite3` object is destroyed while a transaction is open, the transaction
    /// is automatically rolled back.
    fn close(
        &mut self, resource: wasmtime::component::Resource<Sqlite>,
    ) -> wasmtime::Result<Result<(), Errno>> {
        let db_ptr = get_db_state().delete_resource(self.app_name(), resource)?;

        Ok(core::close(db_ptr as *mut _))
    }

    /// Retrieves the numeric result code for the most recent failed `SQLite` operation on
    /// a database connection.
    ///
    /// # Returns
    ///
    /// The numeric result code for the most recent failed `SQLite` operation.
    fn errcode(
        &mut self, resource: wasmtime::component::Resource<Sqlite>,
    ) -> wasmtime::Result<Option<ErrorInfo>> {
        let db_ptr = get_db_state().get_object(self.app_name(), &resource)?;

        Ok(core::errcode(db_ptr as *mut _))
    }

    /// Compiles SQL text into byte-code that will do the work of querying or updating the
    /// database.
    ///
    /// ## Parameters
    ///
    /// - `db`: Database handle.
    /// - `sql`: SQL statement, UTF-8 encoded.
    ///
    /// ## Returns
    ///
    /// A compiled prepared statement that can be executed using `sqlite3_step()`.
    /// If there is an error or the input text contains no SQL (if the input is an empty
    /// string or a comment) then an error code is returned.
    fn prepare(
        &mut self, resource: wasmtime::component::Resource<Sqlite>, sql: String,
    ) -> wasmtime::Result<Result<wasmtime::component::Resource<Statement>, Errno>> {
        let db_ptr = get_db_state().get_object(self.app_name(), &resource)?;

        let result = core::prepare(db_ptr as *mut _, sql.as_str());

        match result {
            Ok(stmt_ptr) => {
                if stmt_ptr.is_null() {
                    Ok(Err(Errno::ReturnedNullPointer))
                } else {
                    let stmt =
                        get_statement_state().create_resource(self.app_name(), stmt_ptr as _)?;

                    Ok(Ok(stmt))
                }
            },
            Err(errno) => Ok(Err(errno)),
        }
    }

    /// Executes an SQL query directly without preparing it into a statement and returns
    /// the result.
    ///
    /// ## Parameters
    ///
    /// - `sql`: SQL statement, UTF-8 encoded.
    fn execute(
        &mut self, resource: wasmtime::component::Resource<Sqlite>, sql: String,
    ) -> wasmtime::Result<Result<(), Errno>> {
        let db_ptr = get_db_state().get_object(self.app_name(), &resource)?;

        Ok(core::execute(db_ptr as *mut _, sql.as_str()))
    }

    fn drop(&mut self, rep: wasmtime::component::Resource<Sqlite>) -> wasmtime::Result<()> {
        if let Ok(db_ptr) = get_db_state().delete_resource(self.app_name(), rep) {
            let _ = core::close(db_ptr as *mut _);
        }

        Ok(())
    }
}
