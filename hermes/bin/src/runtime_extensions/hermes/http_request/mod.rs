//! Http Request extension implementation.

#![allow(unused)]
#![allow(dead_code)]

mod tokio_runtime_task;
mod host;

struct State {
    tokio_rt_handle: tokio_runtime_task::Handle,
}

/// Http Request extension internal state.
static STATE: once_cell::sync::Lazy<State> = once_cell::sync::Lazy::new(|| {
    let tokio_rt_handle = tokio_runtime_task::spawn();

    State { tokio_rt_handle }
});

pub(crate) fn new_context(_ctx: &crate::runtime_context::HermesRuntimeContext) {}

type Payload = u32;

type Error = u32;

/// Send an Http Request
pub(super) fn send(payload: Payload) -> Result<bool, Error> {
    STATE.tokio_rt_handle.send(payload)
}

#[cfg(test)]
mod test {
    use crate::runtime_extensions::hermes::http_request::send;

    #[test]
    fn sending_works() {
        let result = send(24).unwrap();

        assert_eq!(result, true);
    }
}
