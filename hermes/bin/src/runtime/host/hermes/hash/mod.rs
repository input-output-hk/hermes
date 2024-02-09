//! Host - Hash implementations
#![allow(unused_variables)]

use blake2::digest::OutputSizeUser;
use blake2::Blake2bVar;
use blake2::{
    digest::{
        consts::{U20, U32, U48, U64},
        Update, VariableOutput
    },
    Blake2bMac,
};

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
    // Default to 64 bytes Blake2b-512
    let outlen = outlen.unwrap_or(64) as usize;
    let mut output = vec![0u8; outlen];
    // Key is specified, use MAC
    if let Some(k) = key {
        // Type issue, doesn't work
        let mut hasher: Blake2bMac<dyn OutputSizeUser> = match outlen {
            20 => Blake2bMac::<U20>::new_with_salt_and_personal(&k, &[], &[]),
            32 => Blake2bMac::<U32>::new_with_salt_and_personal(&k, &[], &[]),
            48 => Blake2bMac::<U48>::new_with_salt_and_personal(&k, &[], &[]),
            64 => Blake2bMac::<U64>::new_with_salt_and_personal(&k, &[], &[]),
            _ => unreachable!(),
        };
        hasher.update(&buf);
        output = hasher.finalize_fixed().to_vec();
    } else {
        let mut hasher: Blake2bVar = Blake2bVar::new(outlen).unwrap();
        hasher.update(&buf);
        hasher.finalize_variable(&mut output).unwrap();
    }
    return Ok(Bstr::from(output));
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

        let result = blake2b_impl(buf, outlen, None).expect("");
    }
}
