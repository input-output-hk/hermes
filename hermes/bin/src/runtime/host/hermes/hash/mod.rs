//! Host - Hash implementations
#![allow(unused_variables)]

use crate::runtime::extensions::{
    hermes::{
        binary::api::Bstr,
        hash::api::{Errno, Host},
    },
    HermesState, Stateful,
};

/// State
pub(crate) struct State {}

impl Stateful for State {
    fn new(_ctx: &crate::wasm::context::Context) -> Self {
        State {}
    }
}

impl Host for HermesState {
    /// Hash a binary buffer with BLAKE2s
    fn blake2s(
        &mut self, buf: Bstr, outlen: Option<u8>, key: Option<Bstr>,
    ) -> wasmtime::Result<Result<Bstr, Errno>> {
        todo!()
    }

    /// Hash a binary buffer with `BLAKE2b`
    fn blake2b(
        &mut self, buf: Bstr, outlen: Option<u8>, key: Option<Bstr>,
    ) -> wasmtime::Result<Result<Bstr, Errno>> {
        todo!()
    }

    /// Hash a binary buffer with BLAKE3
    fn blake3(
        &mut self, buf: Bstr, outlen: Option<u8>, key: Option<Bstr>,
    ) -> wasmtime::Result<Result<Bstr, Errno>> {
        todo!()
    }
}
