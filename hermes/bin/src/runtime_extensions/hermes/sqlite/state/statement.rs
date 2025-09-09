use std::collections::HashMap;

use crate::runtime_extensions::{
    bindings::hermes::sqlite::api::Statement, hermes::sqlite::state::ObjectPointer,
};

/// Application-specific statement state management
#[derive(Default)]
pub(crate) struct AppStatement {
    /// Map of statement resource IDs to their object pointers
    pub(crate) statements: HashMap<u32, ObjectPointer>,
    /// Next available address id of the resource.
    pub(crate) available_address: u32,
}

impl AppStatement {
    /// Creates a new statement resource and returns a WASM resource handle
    pub(crate) fn create_statement_resource(
        &mut self,
        stmt_ptr: ObjectPointer,
    ) -> wasmtime::component::Resource<Statement> {
        let index = self.available_address;
        self.statements.insert(index, stmt_ptr);
        self.available_address = self.available_address.saturating_add(1);
        wasmtime::component::Resource::new_own(index)
    }

    /// Gets the object pointer for a statement resource
    pub(crate) fn get_object(
        &self,
        resource: &wasmtime::component::Resource<Statement>,
    ) -> Result<&ObjectPointer, wasmtime::Error> {
        let index = resource.rep();
        self.statements
            .get(&index)
            .ok_or_else(|| wasmtime::Error::msg("Statement resource not found"))
    }

    /// Removes and returns the object pointer for a statement resource
    pub(crate) fn delete_resource(
        &mut self,
        resource: &wasmtime::component::Resource<Statement>,
    ) -> Result<ObjectPointer, wasmtime::Error> {
        let index = resource.rep();
        self.statements
            .remove(&index)
            .ok_or_else(|| wasmtime::Error::msg("Statement resource not found"))
    }
}
