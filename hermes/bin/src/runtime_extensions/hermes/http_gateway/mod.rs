//! HTTP Gateway

use gateway_task::spawn;
use rusty_ulid::Ulid;

mod event;
mod gateway_task;
mod routing;

/// State.
pub struct State {
    pub instance: Ulid,
}

///  State.
static STATE: once_cell::sync::Lazy<State> = once_cell::sync::Lazy::new(|| {
    spawn();

    State {
        instance: rusty_ulid::Ulid::generate(),
    }
});

/// New context
pub(crate) fn new_context(_ctx: &crate::runtime_context::HermesRuntimeContext) {
    println!("Instance {:?}", STATE.instance);
}
