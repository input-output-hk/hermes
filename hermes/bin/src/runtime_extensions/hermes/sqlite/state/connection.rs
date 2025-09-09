use crate::runtime_extensions::{
    bindings::hermes::sqlite::api::Sqlite,
    hermes::sqlite::{connection::core::close, state::ObjectPointer},
};

/// Enumeration representing different types of database handles.
///
/// Each handle corresponds to a specific database connection type:
/// - Disk-based (read-only or read-write)
/// - Memory-based (read-only or read-write)
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum DbHandle {
    /// Disk-based database, read-only access
    DiskRO = 0,
    /// Disk-based database, read-write access
    DiskRW = 1,
    /// Memory-based database, read-only access
    MemRO = 2,
    /// Memory-based database, read-write access
    MemRW = 3,
}

impl DbHandle {
    /// Creates a `DbHandle` from readonly and memory flags.
    ///
    /// # Parameters
    ///
    /// - `readonly`: If true, creates a read-only handle
    /// - `memory`: If true, creates a memory-based handle
    ///
    /// # Returns
    ///
    /// The appropriate `DbHandle` variant based on the flags
    pub(crate) fn from_readonly_and_memory(
        readonly: bool,
        memory: bool,
    ) -> Self {
        match (readonly, memory) {
            (true, true) => DbHandle::MemRO,
            (true, false) => DbHandle::DiskRO,
            (false, true) => DbHandle::MemRW,
            (false, false) => DbHandle::DiskRW,
        }
    }
}

impl From<u32> for DbHandle {
    /// Converts a `u32` value to a `DbHandle`.
    ///
    /// # Parameters
    ///
    /// - `value`: The numeric value to convert
    ///
    /// # Returns
    ///
    /// The corresponding `DbHandle` variant, defaulting to `MemRW` for invalid values
    fn from(value: u32) -> Self {
        match value {
            0 => DbHandle::DiskRO,
            1 => DbHandle::DiskRW,
            2 => DbHandle::MemRO,
            _ => DbHandle::MemRW,
        }
    }
}

/// Application-specific database connection state.
///
/// This struct manages all database connections for a single application,
/// including both disk-based and memory-based databases with read-only and
/// read-write access modes.
#[derive(Default)]
#[allow(dead_code)]
pub(crate) struct AppConnections {
    /// Disk-based read-write database connection pointer
    disk_rw: Option<ObjectPointer>,
    /// Disk-based read-only database connection pointer
    disk_ro: Option<ObjectPointer>,
    /// Memory-based read-write database connection pointer
    mem_rw: Option<ObjectPointer>,
    /// Memory-based read-only database connection pointer
    mem_ro: Option<ObjectPointer>,
    /// Mutex guard for disk database access synchronization
    /// Note: This field is reserved for future use when mutex synchronization is needed
    #[allow(dead_code)]
    disk_guard: Option<()>,
    /// Mutex guard for memory database access synchronization
    /// Note: This field is reserved for future use when mutex synchronization is needed
    #[allow(dead_code)]
    mem_guard: Option<()>,
}

impl AppConnections {
    /// Gets a reference to the connection pointer for the specified database handle.
    ///
    /// # Parameters
    ///
    /// - `db_handle`: The database handle to get the connection for
    ///
    /// # Returns
    ///
    /// A reference to the optional connection pointer
    pub(crate) fn get_connection(
        &self,
        db_handle: DbHandle,
    ) -> Option<&ObjectPointer> {
        match db_handle {
            DbHandle::DiskRO => self.disk_ro.as_ref(),
            DbHandle::DiskRW => self.disk_rw.as_ref(),
            DbHandle::MemRO => self.mem_ro.as_ref(),
            DbHandle::MemRW => self.mem_rw.as_ref(),
        }
    }

    /// Gets a mutable reference to the connection pointer for the specified database
    /// handle.
    ///
    /// # Parameters
    ///
    /// - `db_handle`: The database handle to get the mutable connection for
    ///
    /// # Returns
    ///
    /// A mutable reference to the optional connection pointer
    pub(crate) fn get_connection_mut(
        &mut self,
        db_handle: DbHandle,
    ) -> &mut Option<ObjectPointer> {
        match db_handle {
            DbHandle::DiskRO => &mut self.disk_ro,
            DbHandle::DiskRW => &mut self.disk_rw,
            DbHandle::MemRO => &mut self.mem_ro,
            DbHandle::MemRW => &mut self.mem_rw,
        }
    }

    /// Gets a WASM resource handle for the specified database handle, if it exists.
    ///
    /// # Parameters
    ///
    /// - `db_handle`: The database handle to get the resource for
    ///
    /// # Returns
    ///
    /// An optional WASM resource handle for the database connection
    pub(crate) fn get_connection_resource(
        &self,
        db_handle: DbHandle,
    ) -> Option<wasmtime::component::Resource<Sqlite>> {
        self.get_connection(db_handle)
            .map(|_| wasmtime::component::Resource::new_own(db_handle as _))
    }

    /// Creates a new database connection resource and stores the connection pointer.
    ///
    /// # Parameters
    ///
    /// - `db_handle`: The database handle type for the connection
    /// - `db_ptr`: The pointer to the database connection object
    ///
    /// # Returns
    ///
    /// A new WASM resource handle for the database connection
    pub(crate) fn create_connection_resource(
        &mut self,
        db_handle: DbHandle,
        db_ptr: ObjectPointer,
    ) -> wasmtime::component::Resource<Sqlite> {
        *self.get_connection_mut(db_handle) = Some(db_ptr);
        wasmtime::component::Resource::new_own(db_handle as _)
    }
}

impl Drop for AppConnections {
    /// Automatically closes all database connections when the `AppConnections` is
    /// dropped.
    ///
    /// This ensures proper cleanup of database resources and prevents memory leaks.
    fn drop(&mut self) {
        for db_ptr in [self.disk_rw, self.disk_ro, self.mem_rw, self.mem_ro]
            .iter()
            .flatten()
        {
            let _ = close(*db_ptr as _);
        }
    }
}
