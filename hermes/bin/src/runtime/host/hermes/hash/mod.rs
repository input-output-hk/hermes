//! Host - Hash implementations
#![allow(unused_variables)]

use blake2_rfc::blake2b::Blake2b;

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

fn blake2b_impl(buf: Bstr, outlen: Option<u8>, key: Option<Bstr>) -> Result<Bstr, Errno> {
    let outlen = match outlen {
        Some(len) => len as usize,
        None => 64, // Default output length is 64 bytes, blake2b-512
    };

    let key = match key {
        Some(k) => k,
        None => (&[]).to_vec(),
    };

    let mut hasher = Blake2b::with_key(outlen, &key);
    hasher.update(&buf);
    Ok(hasher.finalize().as_bytes().to_vec())
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
        Ok(blake2b_impl(buf, outlen, key))
    }

    /// Hash a binary buffer with BLAKE3
    fn blake3(
        &mut self, buf: Bstr, outlen: Option<u8>, key: Option<Bstr>,
    ) -> wasmtime::Result<Result<Bstr, Errno>> {
        todo!()
    }
}

#[cfg(test)]
mod tests_blake2b {
    use super::*;
    use hex_literal::hex;

    #[test]
    fn blake2b_512_with_key() {
        // Setup
        let buf = Bstr::from("test test");
        let key = Bstr::from("key");
        let outlen = Some(64);

        let result =
            blake2b_impl(buf, outlen, Some(key)).expect("Failed to hash blake2b-512 with key");

        assert_eq!(
            result.as_ref(),
            hex!("c28029cbab4e11d759e971d7e2a13dbe9ef60d2fa539cc03138b0432c3fdb2757b6c87383bd1074f5533c0c2ad2a5d2ac71bbd96f0f8fbb4c3ba0d4abb309115")
        );
    }

    #[test]
    fn blake2b_512_with_no_key() {
        // Setup
        let buf = Bstr::from("test test");
        let outlen = Some(64);

        let result =
            blake2b_impl(buf, outlen, None).expect("Failed to hash blake2b-512 without key");

        assert_eq!(
            result.as_ref(),
            hex!("8e27b2481dd1fe73d598104c03b1f67da60725abb73cf66e400177d73aee01e74b93f55adda27b0ad92e22e284b5e0cc95ad81b04b496bd58c4ae6bca5f56196")
        );
    }

    #[test]
    fn blake2b_256_with_no_key() {
        // Setup
        let buf = Bstr::from("test test");
        let outlen = Some(32);

        let result =
            blake2b_impl(buf, outlen, None).expect("Failed to hash blake2b-256 without key");

        assert_eq!(
            result.as_ref(),
            hex!("7f3dc1170e7017a1643d84d102429c4c7aec4ca99c016c32af18af997fed51f1")
        );
    }
    #[test]
    fn blake2b_512_with_default_outlen_no_key() {
        // Setup
        let buf = Bstr::from("test test");

        let result = blake2b_impl(buf, None, None)
            .expect("Failed to hash blake2b-512 default outlen without key");

        assert_eq!(
            result.as_ref(),
            hex!("8e27b2481dd1fe73d598104c03b1f67da60725abb73cf66e400177d73aee01e74b93f55adda27b0ad92e22e284b5e0cc95ad81b04b496bd58c4ae6bca5f56196")
        );
    }

    #[test]
    fn blake2b_0_outlen() {
        // Setup
        let buf = Bstr::from("test test");
        let outlen = Some(0);

        let result =
            blake2b_impl(buf, outlen, None).expect("");
    }
}
