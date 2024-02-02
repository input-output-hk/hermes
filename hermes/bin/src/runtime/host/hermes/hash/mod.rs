//! Host - Hash implementations
//!
#![allow(unused_variables)]

use crate::runtime::extensions::hermes::binary::api::Bstr;
use crate::runtime::extensions::hermes::hash::api::{Errno, Host};

/// State of the hash
struct HashState {}

impl Host for HashState {
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
