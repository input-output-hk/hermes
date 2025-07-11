//! Http Request extension implementation.

#![allow(unused)]
#![allow(dead_code)]

use std::sync::{Arc, LazyLock, OnceLock, RwLock};

use tokio_rustls::TlsConnector;

mod event;
mod host;
mod tokio_runtime_task;

struct State {
    pub tokio_rt_handle: tokio_runtime_task::TokioTaskHandle,
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

pub(crate) fn new_context(_ctx: &crate::runtime_context::HermesRuntimeContext) {}

type Error = u32;

#[cfg(test)]
mod test {}
