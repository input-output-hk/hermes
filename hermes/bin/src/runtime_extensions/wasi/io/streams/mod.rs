//! IO Streams runtime extension implementation.

mod host;
mod state;

pub(crate) use state::{get_input_streams_state, get_output_streams_state};

/// Advise Runtime Extensions of a new context
pub(crate) fn new_context(ctx: &crate::runtime_context::HermesRuntimeContext) {
    get_input_streams_state().add_app(ctx.app_name().clone());
    get_output_streams_state().add_app(ctx.app_name().clone());
}
