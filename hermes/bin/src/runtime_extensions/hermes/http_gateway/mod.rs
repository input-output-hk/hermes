//! HTTP Gateway

use std::sync::Arc;

use dashmap::DashMap;
use gateway_task::spawn;
use rusty_ulid::Ulid;

use crate::{app::HermesAppName, vfs::Vfs};

mod event;
mod gateway_task;
/// Gateway routing logic
mod routing;

/// State.
pub(crate) struct State {
    /// UID for wasm instance
    pub(crate) instance: Ulid,
    /// Virtual file system for each app
    pub(crate) vfs: DashMap<HermesAppName, Arc<Vfs>>,
}

///  State.
static STATE: once_cell::sync::Lazy<State> = once_cell::sync::Lazy::new(|| {
    spawn();

    State {
        instance: rusty_ulid::Ulid::generate(),
        vfs: DashMap::new(),
    }
});

/// New context
pub(crate) fn new_context(ctx: &crate::runtime_context::HermesRuntimeContext) {
    println!(
        "Instance {:?}\n
        App name {:?}",
        STATE.instance,
        ctx.app_name()
    );
    STATE.vfs.insert(ctx.app_name().clone(), ctx.vfs());
}
