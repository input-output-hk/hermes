//! Host - Hash implementations

use crate::{
    runtime_extensions::{
        bindings::hermes::{
            binary::api::Bstr,
            hash::api::{Errno, Host},
        },
        state::{Context, Stateful},
    },
    state::HermesState,
};

/// State
pub(crate) struct State {}

impl Stateful for State {
    fn new(_ctx: &Context) -> Self {
        State {}
    }
}

impl Host for HermesState {
    /// Hash a binary buffer with BLAKE2s
    fn blake2s(
        &mut self, _buf: Bstr, _outlen: Option<u8>, _key: Option<Bstr>,
    ) -> wasmtime::Result<Result<Bstr, Errno>> {
        todo!()
    }

    /// Hash a binary buffer with `BLAKE2b`
    fn blake2b(
        &mut self, _buf: Bstr, _outlen: Option<u8>, _key: Option<Bstr>,
    ) -> wasmtime::Result<Result<Bstr, Errno>> {
        todo!()
    }

    /// Hash a binary buffer with BLAKE3
    fn blake3(
        &mut self, _buf: Bstr, _outlen: Option<u8>, _key: Option<Bstr>,
    ) -> wasmtime::Result<Result<Bstr, Errno>> {
        todo!()
    }
}
