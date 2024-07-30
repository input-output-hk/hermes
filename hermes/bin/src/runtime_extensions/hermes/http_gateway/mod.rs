//! HTTP Gateway

use std::sync::OnceLock;

use gateway_task::spawn;
use rusty_ulid::Ulid;

use crate::vfs::Vfs;

mod event;
mod gateway_task;
/// Gateway routing logic
mod routing;

/// Virtual file system
static VFS: OnceLock<Vfs> = OnceLock::new();

/// State.
pub(crate) struct State {
    /// UID for wasm instance
    pub(crate) instance: Ulid,
}

///  State.
static STATE: once_cell::sync::Lazy<State> = once_cell::sync::Lazy::new(|| {
    spawn();

    State {
        instance: rusty_ulid::Ulid::generate(),
    }
});

/// New context
pub(crate) fn new_context(ctx: &crate::runtime_context::HermesRuntimeContext) {
    VFS.get_or_init(|| ctx.vfs().clone());
    println!("Instance {:?}", STATE.instance);
}
