//! Filesystem runtime extension implementation.

use super::state::STATE;

mod host;

/// Advise Runtime Extensions of a new context
pub(crate) fn new_context(ctx: &crate::runtime_context::HermesRuntimeContext) {
    let mut app_state = STATE.get_mut(ctx.app_name());
    app_state.put_preopen_dir("/".to_string(), ctx.vfs().root().clone());
}
