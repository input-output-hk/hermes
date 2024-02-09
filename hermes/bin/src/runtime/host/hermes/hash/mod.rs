//! Host - Hash implementations
#![allow(unused_variables)]

// use blake2::digest::{consts::{U32}, FixedOutput, Update, VariableOutput};
// use blake2::{Blake2bVar};
use blake2_rfc::blake2b::Blake2b;
use hex_literal::hex;

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
    let outlen = outlen.unwrap_or(64) as usize;
    // let mut output = vec![0u8; outlen];

    let key = match key {
        Some(k) => k,
        None => (&[]).to_vec(),
    };

    let mut ctx = Blake2b::with_key(outlen, &key);
    ctx.update(&buf);
    return Ok(ctx.finalize().as_bytes().to_vec());

    // Default to 64 bytes Blake2b-512
    // let outlen = outlen.unwrap_or(64) as usize;
    // let mut output = vec![0u8; outlen];
    // Key is specified, use MAC
    // if let Some(k) = key {
    // let mut hasher = Blake2bMac::<U32>::new_with_salt_and_personal(&k, &[], &[]).unwrap();
    // hasher.update(&buf);
    // output = hasher.finalize_fixed().to_vec();
    //     let ctx = Blake2::with_key(outlen, key);
    //     ctx.update(buf);
    //     let output = ctx.finalize();
    // } else {
    //     let mut hasher = Blake2bVar::new(outlen).unwrap();
    //     hasher.update(&buf);
    //     hasher.finalize_variable(&mut output).unwrap();
    // };
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

#[test]
fn test_blake2b() {
    // Setup
    let context = crate::wasm::context::Context::new("Test".to_string());
    let state = crate::runtime::host::hermes::State::new(&context);
    let mut hermes_state = HermesState::new(&context);

    let buf = Bstr::from("test test");
    let key = Bstr::from("key");
    let outlen = Some(64);

    let result = hermes_state.blake2b(buf, outlen, Some(key));

    // Assert the result
    match result {
        Ok(Ok(result_buf)) => {
            // Hash without key, should work properly
            assert_ne!(
                result_buf.as_ref(),
                hex!("c28029cbab4e11d759e971d7e2a13dbe9ef60d2fa539cc03138b0432c3fdb2757b6c87383bd1074f5533c0c2ad2a5d2ac71bbd96f0f8fbb4c3ba0d4abb309115")
            );
        },
        Ok(Err(errno)) => {
            panic!("Error returned: {:?}", errno);
        },
        Err(err) => {
            panic!("Function failed with error: {:?}", err);
        },
    }
}
#[test]
fn test_blake2b_ja() {
    // Setup
    let context = crate::wasm::context::Context::new("Test".to_string());
    let state = crate::runtime::host::hermes::State::new(&context);
    let mut hermes_state = HermesState::new(&context);

    let buf = Bstr::from("test test");
    let key = Bstr::from("key");
    let outlen = Some(64);

    let result = hermes_state.blake2b(buf, outlen, None);

    // Assert the result
    match result {
        Ok(Ok(result_buf)) => {
            // Hash without key, should work properly
            assert_eq!(
                result_buf.as_ref(),
                hex!("8e27b2481dd1fe73d598104c03b1f67da60725abb73cf66e400177d73aee01e74b93f55adda27b0ad92e22e284b5e0cc95ad81b04b496bd58c4ae6bca5f56196")
            );
        },
        Ok(Err(errno)) => {
            panic!("Error returned: {:?}", errno);
        },
        Err(err) => {
            panic!("Function failed with error: {:?}", err);
        },
    }
}
