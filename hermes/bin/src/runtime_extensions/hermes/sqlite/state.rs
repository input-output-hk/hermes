use std::hash::{Hash, Hasher};

use dashmap::DashMap;
use once_cell::sync::Lazy;
use tokio::{
    runtime::Builder,
    sync::{mpsc, oneshot},
};

use crate::{app::HermesAppName, runtime_extensions::bindings::hermes::sqlite::api::Sqlite};

/// Map of app name to resource holder
type State = DashMap<HermesAppName, InternalState>;

/// Global state to hold the resources.
static SQLITE_INTERNAL_STATE: Lazy<State> = Lazy::new(DashMap::new);

struct InternalState {
  
}

impl InternalState {
    fn new() -> Self {
        Self {

        }
    }

    fn drop(id: u32) {
      
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
  if let Some(res_holder) = SQLITE_INTERNAL_STATE.get(app_name) {
      return res_holder.get_resource_from_id(id);
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