use std::hash::{Hash, Hasher};

use dashmap::DashMap;
use once_cell::sync::Lazy;
use tokio::{
    runtime::Builder,
    sync::{mpsc, oneshot},
};

use crate::{app::HermesAppName, runtime_extensions::bindings::hermes::sqlite::api::{Sqlite, Statement}};

/// Map of app name to resource holder
type State = DashMap<HermesAppName, InternalState>;

/// Global state to hold the resources.
static SQLITE_INTERNAL_STATE: Lazy<State> = Lazy::new(DashMap::new);

struct InternalState {
  sqlite_object: Sqlite,
  sqlite_id: u32,
  id_to_statement_map: DashMap<u32, Statement>,
}

impl InternalState {
    fn new() -> Self {
        Self {
          id_to_sqlite_map: DashMap::new(),
          id_to_statement_map: DashMap::new()
        }
    }

    fn drop(&mut self, id: u32) -> Option<u32> {
        // Check if the resource exists in id_to_resource_map.
        if let Some(resource) = self.get_resource_from_id(id) {
            // Check if the id exists in resource_to_id_map.
            if let Some(associated_id) = self.get_id_from_resource(&resource) {
                // The id should be the same.
                if associated_id == id {
                    // Remove the resource from both maps.
                    if let Some(r) = self.id_to_resource_map.remove(&id) {
                        self.resource_to_id_map.remove(&WrappedXPrv(r.1));
                        return Some(associated_id);
                    }
                }
            }
        }
        None
    }
}

/// Get the state.
pub(super) fn get_state() -> &'static State {
    &SQLITE_INTERNAL_STATE
}

/// Set the state according to the app context.
pub(crate) fn set_state(app_name: HermesAppName) {
    SQLITE_INTERNAL_STATE.insert(app_name, InternalState::new());
}

/// Get the resource from the state using id if possible.
pub(crate) fn get_resource(app_name: &HermesAppName, id: u32) -> Option<Sqlite> {
    if let Some(internal_state) = SQLITE_INTERNAL_STATE.get(app_name) {
        return internal_state.get_resource_from_id(id);
    }
    None
}

/// Delete the resource from the state using id if possible.
pub(crate) fn delete_resource(app_name: &HermesAppName, id: u32) -> Option<u32> {
    if let Some(mut res_holder) = SQLITE_INTERNAL_STATE.get_mut(app_name) {
        return res_holder.drop(id);
    }
    None
}
