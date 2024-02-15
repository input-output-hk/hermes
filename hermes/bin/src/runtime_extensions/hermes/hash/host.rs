//! Hash host implementation for WASM runtime.

use crate::{
    runtime_extensions::bindings::hermes::{
        binary::api::Bstr,
        hash::api::{Errno, Host},
    },
    state::HermesState,
};

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
