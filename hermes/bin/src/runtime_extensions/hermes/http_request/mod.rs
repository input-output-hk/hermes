//! Http Request extension implementation.

use std::sync::OnceLock;

use tokio_rustls::TlsConnector;

/// HTTP Request events module.
mod event;
/// HTTP Request host module.
mod host;
/// Tokio runtime task module for handling HTTP requests.
mod tokio_runtime_task;

/// Http Request extension state.
struct State {
    /// Tokio runtime task handle for sending HTTP requests.
    pub tokio_rt_handle: tokio_runtime_task::TokioTaskHandle,
    /// TLS connector for secure HTTP requests.
    pub tls_connector: OnceLock<TlsConnector>,
}

/// Http Request extension internal state.
static STATE: once_cell::sync::Lazy<State> = once_cell::sync::Lazy::new(|| {
    let tokio_rt_handle = tokio_runtime_task::spawn();

    State {
        tokio_rt_handle,
        tls_connector: OnceLock::new(),
    }
});

/// New context for the HTTP Request extension.
pub(crate) fn new_context(_ctx: &crate::runtime_context::HermesRuntimeContext) {}

#[cfg(test)]
mod test {}
