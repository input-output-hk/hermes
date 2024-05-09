//! `SQLite` runtime extension implementation.

mod connection;
mod host;
mod state;
mod statement;

/// Advise Runtime Extensions of a new context
pub(crate) fn new_context(ctx: &crate::runtime_context::HermesRuntimeContext) {
    connection::new_context(ctx);
    statement::new_context(ctx);
}

use libsqlite3_sys::*;

use crate::runtime_extensions::bindings::hermes::sqlite::api::Errno;

impl From<i32> for Errno {
    fn from(value: i32) -> Self {
        match value {
            SQLITE_ERROR => Errno::Error,
            SQLITE_INTERNAL => Errno::Internal,
            SQLITE_PERM => Errno::Perm,
            SQLITE_ABORT => Errno::Abort,
            SQLITE_BUSY => Errno::Busy,
            SQLITE_LOCKED => Errno::Locked,
            SQLITE_NOMEM => Errno::Nomem,
            SQLITE_READONLY => Errno::Readonly,
            SQLITE_INTERRUPT => Errno::Interrupt,
            SQLITE_IOERR => Errno::Ioerr,
            SQLITE_CORRUPT => Errno::Corrupt,
            SQLITE_NOTFOUND => Errno::Notfound,
            SQLITE_FULL => Errno::Full,
            SQLITE_CANTOPEN => Errno::Cantopen,
            SQLITE_PROTOCOL => Errno::Protocol,
            SQLITE_EMPTY => Errno::Empty,
            SQLITE_SCHEMA => Errno::Schema,
            SQLITE_TOOBIG => Errno::Toobig,
            SQLITE_CONSTRAINT => Errno::Constraint,
            SQLITE_MISMATCH => Errno::Mismatched,
            SQLITE_MISUSE => Errno::Misuse,
            SQLITE_NOLFS => Errno::Nolfs,
            SQLITE_AUTH => Errno::Auth,
            SQLITE_FORMAT => Errno::Format,
            SQLITE_RANGE => Errno::Range,
            SQLITE_NOTADB => Errno::Notadb,
            SQLITE_NOTICE => Errno::Notice,
            SQLITE_WARNING => Errno::Warning,
            _ => Errno::Error,
        }
    }
}
