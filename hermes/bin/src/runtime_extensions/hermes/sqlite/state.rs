//! Internal state implementation for the `SQLite` module.

use std::collections::HashMap;

use dashmap::{mapref::one::RefMut, DashMap};
use once_cell::sync::Lazy;

use crate::app::HermesAppName;

/// Represents an individual state for a particular object.
#[derive(Debug)]
pub(crate) struct ResourceObjectState {
    /// A map holding key-value pairs of an object ID and a value.
    id_map: HashMap<u32, usize>,
    /// The current incremental state of ID.
    current_id: Option<u32>,
}

/// Represents the state of resources.
pub(crate) struct ResourceState {
    /// The state of database object.
    db_ptr_state: ResourceObjectState,
    /// The state of database statement object.
    stmt_ptr_state: ResourceObjectState,
}

impl ResourceObjectState {
    /// Create a new `ResourceObjectState` with initial state.
    fn new() -> Self {
        Self {
            id_map: HashMap::new(),
            current_id: None,
        }
    }

    /// Adds a value into the resource. If it does not exist, allocate one and returns the
    /// new created key ID. In case of the key ID is running out of numbers, returns
    /// `None`.
    pub(super) fn allocate_object(&mut self, item_ptr: usize) -> Option<u32> {
        if let Some((existing_id, _)) = self.id_map.iter().find(|(_, val)| val == &&item_ptr) {
            Some(*existing_id)
        } else {
            let (new_id, is_overflow) = self
                .current_id
                .map_or_else(|| (0, false), |id| id.overflowing_add(1));

            if is_overflow {
                None
            } else {
                self.id_map.insert(new_id, item_ptr);
                self.current_id = Some(new_id);
                Some(new_id)
            }
        }
    }

    /// Retrieves a value according to its key ID.
    pub(super) fn get_object_by_id(&self, id: u32) -> Option<usize> {
        self.id_map.get(&id).map(ToOwned::to_owned)
    }

    /// Deletes a value according to its key ID, and returns the removed value if exists.
    pub(super) fn delete_object_by_id(&mut self, id: u32) -> Option<usize> {
        self.id_map.remove(&id)
    }
}

impl ResourceState {
    /// Create a new `ResourceState` with initial state.
    pub(super) fn new() -> Self {
        Self {
            db_ptr_state: ResourceObjectState::new(),
            stmt_ptr_state: ResourceObjectState::new(),
        }
    }

    /// Gets the state for managing database objects.
    pub(super) fn get_db_state(&mut self) -> &mut ResourceObjectState {
        &mut self.db_ptr_state
    }

    /// Gets the state for managing statement objects.
    pub(super) fn get_stmt_state(&mut self) -> &mut ResourceObjectState {
        &mut self.stmt_ptr_state
    }
}

/// Map of app name to resource holder
type State = DashMap<HermesAppName, ResourceState>;

/// Global state to hold `SQLite` resources.
static SQLITE_INTERNAL_STATE: Lazy<State> = Lazy::new(DashMap::new);

/// Represents the internal state object for `SQLite` module.
pub(crate) struct InternalState;

impl InternalState {
    /// Set the state according to the app context.
    pub(crate) fn get_or_create_resource<'a>(
        app_name: HermesAppName,
    ) -> RefMut<'a, HermesAppName, ResourceState> {
        SQLITE_INTERNAL_STATE
            .entry(app_name)
            .or_insert_with(ResourceState::new)
    }
}
