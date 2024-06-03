// cspell: words errcode errmsg

/// ! Core functionality implementation for `SQLite` connection object.
use libsqlite3_sys::{
    sqlite3, sqlite3_close_v2, sqlite3_errcode, sqlite3_errmsg, sqlite3_finalize,
    sqlite3_prepare_v3, sqlite3_step, sqlite3_stmt, SQLITE_DONE, SQLITE_OK,
};
use stringzilla::StringZilla;

use crate::runtime_extensions::bindings::hermes::sqlite::api::{Errno, ErrorInfo};

/// Splits a given SQL string into individual commands, ensuring that semicolons
/// within string literals are not treated as command separators.
///
/// # Arguments
///
/// * `sql` - A string slice that holds the SQL statements to be split into individual
///   commands.
///
/// # Returns
///
/// * `Vec<String>` - A vector of strings, each containing an individual SQL command.
fn split_sql_commands(sql: &str) -> Vec<String> {
    let mut in_single_quote = false;
    let mut in_double_quote = false;
    let mut escape = false;
    let mut current_command = String::new();
    let mut commands = Vec::new();

    for c in sql.chars() {
        current_command.push(c);

        match c {
            '\'' if !in_double_quote && !escape => in_single_quote = !in_single_quote,
            '"' if !in_single_quote && !escape => in_double_quote = !in_double_quote,
            '\\' if !escape => escape = true,
            ';' if !in_single_quote && !in_double_quote => {
                // If not inside a string literal, push the current command to the vector
                commands.push(current_command.trim().to_string());
                current_command.clear();
            },
            _ => {
                escape = false;
            },
        }
    }

    // Push the last command if it's not empty
    if !current_command.trim().is_empty() {
        commands.push(current_command.trim().to_string());
    }

    commands
}

/// Checks if the provided SQL string contains a `PRAGMA` statement.
/// Generally, `PRAGMA` is intended for internal use only.
pub(crate) fn validate_sql(sql: &str) -> bool {
    sql.to_uppercase().sz_find("PRAGMA ".as_bytes()).is_some()
}

/// Closes a database connection, destructor for `sqlite3`.
pub(crate) fn close(db_ptr: *mut sqlite3) -> Result<(), Errno> {
    let rc = unsafe { sqlite3_close_v2(db_ptr) };

    if rc != SQLITE_OK {
        Err(Errno::Sqlite(rc))
    } else {
        Ok(())
    }
}

/// Retrieves runtime status information about a single database connection.
pub(crate) fn errcode(db_ptr: *mut sqlite3) -> Option<ErrorInfo> {
    let (error_code, error_msg) = unsafe { (sqlite3_errcode(db_ptr), sqlite3_errmsg(db_ptr)) };

    if error_code == SQLITE_OK {
        return None;
    }

    let message = unsafe {
        std::ffi::CStr::from_ptr(error_msg)
            .to_str()
            .map(std::borrow::ToOwned::to_owned)
            .ok()
    };

    message.map(|message| {
        ErrorInfo {
            code: error_code,
            message,
        }
    })
}

/// Compiles SQL text into byte-code that will do the work of querying or updating the
/// database.
pub(crate) fn prepare(db_ptr: *mut sqlite3, sql: &str) -> Result<*mut sqlite3_stmt, Errno> {
    if validate_sql(sql) {
        return Err(Errno::ForbiddenPragmaCommand);
    }

    let sql_cstring = std::ffi::CString::new(sql).map_err(|_| Errno::ConvertingCString)?;

    let mut stmt_ptr: *mut sqlite3_stmt = std::ptr::null_mut();

    let n_byte = sql_cstring.as_bytes_with_nul().len();

    let rc = unsafe {
        sqlite3_prepare_v3(
            db_ptr,
            sql_cstring.as_ptr(),
            n_byte as i32,
            0,
            &mut stmt_ptr,
            std::ptr::null_mut(),
        )
    };

    if rc != SQLITE_OK {
        Err(Errno::Sqlite(rc))
    } else {
        Ok(stmt_ptr)
    }
}

/// Executes an SQL query directly without preparing it into a statement and returns
/// the result.
pub(crate) fn execute(db_ptr: *mut sqlite3, sql: &str) -> Result<(), Errno> {
    let commands = split_sql_commands(sql);

    for command in commands {
        let stmt_ptr = prepare(db_ptr, command.as_str())?;

        let rc = unsafe { sqlite3_step(stmt_ptr) };
        if rc != SQLITE_DONE {
            return Err(Errno::Sqlite(rc));
        }

        let rc = unsafe { sqlite3_finalize(stmt_ptr) };
        if rc != SQLITE_OK {
            return Err(Errno::Sqlite(rc));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        app::HermesAppName,
        runtime_extensions::hermes::sqlite::{core::open, statement::core::finalize},
    };

    const TMP_DIR: &str = "tmp-dir";

    fn init() -> Result<*mut sqlite3, Errno> {
        let app_name = HermesAppName(String::from(TMP_DIR));

        open(false, true, app_name)
    }

    #[test]
    fn test_validate_pragma() -> Result<(), Errno> {
        let db_ptr = init()?;

        let sql = "PRAGMA page_size;";
        let stmt_ptr = prepare(db_ptr, sql);

        assert!(matches!(stmt_ptr, Err(Errno::ForbiddenPragmaCommand)));

        let sql = "pragma page_size;";
        let stmt_ptr = prepare(db_ptr, sql);

        assert!(matches!(stmt_ptr, Err(Errno::ForbiddenPragmaCommand)));

        close(db_ptr)
    }

    #[test]
    fn test_prepare_simple() -> Result<(), Errno> {
        let db_ptr = init()?;

        let sql = "SELECT 1;";

        let stmt_ptr = prepare(db_ptr, sql)?;

        finalize(stmt_ptr)?;

        close(db_ptr)
    }

    #[test]
    fn test_execute_create_schema_simple() -> Result<(), Errno> {
        let db_ptr = init()?;

        let create_table_sql = r"
            CREATE TABLE IF NOT EXISTS people (
                id INTEGER PRIMARY KEY,
                name TEXT,
                age INTEGER
            );
        ";

        execute(db_ptr, create_table_sql)?;

        close(db_ptr)
    }

    #[test]
    fn test_err_info() -> Result<(), Errno> {
        let db_ptr = init()?;

        let insert_user_sql = r"
            INSERT INTO user(name, email) VALUES('testing', 'sample');
        ";
        let result = execute(db_ptr, insert_user_sql);

        let err_info = errcode(db_ptr);

        close(db_ptr)?;

        assert!(result.is_err());

        if let Some(err_info) = err_info {
            assert_eq!(err_info.code, 1);
            assert_eq!(err_info.message, String::from("no such table: user"));
        } else {
            panic!();
        }

        Ok(())
    }

    #[test]
    fn test_execute_create_schema_multiple() -> Result<(), Errno> {
        let db_ptr = init()?;

        let create_table_sql = r"
            CREATE TABLE user (
                id INTEGER PRIMARY KEY,
                name TEXT NOT NULL,
                email TEXT NOT NULL UNIQUE
            );
            CREATE TABLE profile (
                id INTEGER PRIMARY KEY,
                bio TEXT NOT NULL
            );
        ";
        execute(db_ptr, create_table_sql)?;

        let insert_user_sql = r"
            INSERT INTO user(name, email) VALUES('testing', 'sample');
        ";
        execute(db_ptr, insert_user_sql)?;

        let insert_order_sql = r"
            INSERT INTO profile(bio) VALUES('testing');
        ";
        execute(db_ptr, insert_order_sql)?;

        let err_info = errcode(db_ptr);

        assert!(err_info.is_none());

        close(db_ptr)
    }

    #[test]
    fn test_close_simple() -> Result<(), Errno> {
        let db_ptr = init()?;

        close(db_ptr)
    }
}
