//! WASI runtime extension state.

use dashmap::{
    mapref::one::{Ref, RefMut},
    DashMap,
};
use once_cell::sync::Lazy;

use super::context::WasiContext;
use crate::app::HermesAppName;

/// Holds WASI context for each application.
pub(super) struct State(DashMap<HermesAppName, WasiContext>);

impl State {
    /// Creates a new state.
    pub(super) fn new() -> Self {
        Self(DashMap::new())
    }

    /// Returns a mutable reference to the context of the given application.
    ///
    /// Creates a default one if it doesn't exist.
    pub(super) fn get_mut(&self, app_name: &HermesAppName) -> RefMut<HermesAppName, WasiContext> {
        if let Some(r) = self.0.get_mut(app_name) {
            r
        } else {
            self.0.entry(app_name.clone()).or_default()
        }
    }

    /// Returns a reference to the context of the given application.
    ///
    /// Creates a default one if it doesn't exist.
    pub(super) fn get(&self, app_name: &HermesAppName) -> Ref<HermesAppName, WasiContext> {
        if let Some(r) = self.0.get(app_name) {
            r
        } else {
            self.0.entry(app_name.clone()).or_default().downgrade()
        }
    }
}

/// WASI runtime extension state.
pub(super) static STATE: Lazy<State> = Lazy::new(State::new);
