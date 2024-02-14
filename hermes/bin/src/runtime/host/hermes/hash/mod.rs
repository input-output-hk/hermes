//! Host - Hash implementations

use crate::{
    runtime::extensions::{
        bindings::hermes::{
            binary::api::Bstr,
            hash::api::{Errno, Host},
        },
        state::{Context, Stateful},
    },
    state::HermesState,
};

mod blake2b;

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
        &mut self, _buf: Bstr, _outlen: Option<u8>,
    ) -> wasmtime::Result<Result<Bstr, Errno>> {
        todo!()
    }

    /// Hash a binary buffer with `BLAKE2s` with `MAC` (Message Authentication Code) mode
    fn blake2smac(
        &mut self, _buf: Bstr, _outlen: Option<u8>, _key: Bstr, _salt: Option<Bstr>,
        _persona: Option<Bstr>,
    ) -> wasmtime::Result<Result<Bstr, Errno>> {
        todo!()
    }

    /// Hash a binary buffer with `BLAKE2b`
    fn blake2b(&mut self, buf: Bstr, outlen: Option<u8>) -> wasmtime::Result<Result<Bstr, Errno>> {
        blake2b::blake2b_impl(&buf, outlen)
            .map(Ok)
            .map_err(Into::into)
    }

    /// Hash a binary buffer with `BLAKE2b` with `MAC` (Message Authentication Code) mode
    fn blake2bmac(
        &mut self, buf: Bstr, outlen: Option<u8>, key: Bstr, salt: Option<Bstr>,
        personal: Option<Bstr>,
    ) -> wasmtime::Result<Result<Bstr, Errno>> {
        blake2b::blake2bmac_impl(&buf, outlen, &key, salt, personal)
            .map(Ok)
            .map_err(Into::into)
    }

    /// Hash a binary buffer with BLAKE3
    fn blake3(
        &mut self, _buf: Bstr, _outlen: Option<u8>, _key: Option<Bstr>,
    ) -> wasmtime::Result<Result<Bstr, Errno>> {
        todo!()
    }
}
