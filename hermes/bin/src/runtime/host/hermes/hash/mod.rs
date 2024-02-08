//! Host - Hash implementations
#![allow(unused_variables)]

use blake2::digest::{Update, VariableOutput};
use blake2::{Blake2bVar, Blake2bVarCore};
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
        // Recheck this implementation might need to use Blake2sMac256 for adding the key
        let mut v = Blake2bVarCore::new_with_params(salt, persona, key_size, output_size);
        let mut hasher = Blake2bVar::new(32).unwrap();
        hasher.update(&buf);
        let mut buf = [0u8; 32];
        hasher.finalize_variable(&mut buf).unwrap();
        return Ok(Ok(Bstr::from(buf.to_vec())));
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
    let outlen = Some(32);
    let key = Some(Bstr::from("key"));

    let result = hermes_state.blake2b(buf, outlen, key);

    // Assert the result
    match result {
        Ok(Ok(result_buf)) => {
            // Hash without key, should work properly
            assert_eq!(
                result_buf.as_ref(),
                hex!("7f3dc1170e7017a1643d84d102429c4c7aec4ca99c016c32af18af997fed51f1")
            );
        },
        Ok(Err(errno)) => {
            panic!("Error returned: {:?}", errno);
        },
        Err(err) => {
            panic!("Function failed with error: {:?}", err);
        }
    }
}
