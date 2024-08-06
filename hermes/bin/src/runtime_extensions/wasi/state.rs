use dashmap::{
    mapref::one::{Ref, RefMut},
    DashMap,
};
use once_cell::sync::Lazy;

use super::context::WasiContext;
use crate::app::HermesAppName;

pub(super) struct State(DashMap<HermesAppName, WasiContext>);

impl State {
    pub(super) fn new() -> Self {
        Self(DashMap::new())
    }

    pub(super) fn get_mut(&self, app_name: &HermesAppName) -> RefMut<HermesAppName, WasiContext> {
        match self.0.get_mut(app_name) {
            Some(r) => r,
            None => {
                self.0.insert(app_name.clone(), WasiContext::default());

                let Some(r) = self.0.get_mut(app_name) else {
                    unreachable!()
                };

                r
            },
        }
    }

    pub(super) fn get(&self, app_name: &HermesAppName) -> Ref<HermesAppName, WasiContext> {
        match self.0.get(app_name) {
            Some(r) => r,
            None => {
                self.0.insert(app_name.clone(), WasiContext::default());

                let Some(r) = self.0.get(app_name) else {
                    unreachable!()
                };

                r
            },
        }
    }
}

pub(super) static STATE: Lazy<State> = Lazy::new(State::new);
