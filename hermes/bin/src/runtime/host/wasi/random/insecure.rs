//! Insecure RNG

use crate::runtime::extensions::{wasi::random::insecure::Host, HermesState, NewState};

/// WASI State
pub(crate) struct State {}

impl NewState for State {
    fn new(_ctx: &crate::wasm::context::Context) -> Self {
        Self {}
    }
}

impl Host for HermesState {
    #[doc = " Return `len` insecure pseudo-random bytes."]
    #[doc = " "]
    #[doc = " This function is not cryptographically secure. Do not use it for"]
    #[doc = " anything related to security."]
    #[doc = " "]
    #[doc = " There are no requirements on the values of the returned bytes, however"]
    #[doc = " implementations are encouraged to return evenly distributed values with"]
    #[doc = " a long period."]
    fn get_insecure_random_bytes(&mut self, len: u64) -> wasmtime::Result<Vec<u8>> {
        todo!()
    }

    #[doc = " Return an insecure pseudo-random `u64` value."]
    #[doc = " "]
    #[doc = " This function returns the same type of pseudo-random data as"]
    #[doc = " `get-insecure-random-bytes`, represented as a `u64`."]
    fn get_insecure_random_u64(&mut self) -> wasmtime::Result<u64> {
        todo!()
    }
}
