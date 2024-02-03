//! Random RNG

use crate::runtime::extensions::wasi::random::random::Host;
use crate::runtime::extensions::{HermesState, NewState};

/// WASI State
pub(crate) struct State {}

impl NewState for State {
    fn new(_ctx: &crate::wasm::context::Context) -> Self {
        Self {}
    }
}

impl Host for HermesState {
    #[doc = " Return `len` cryptographically-secure random or pseudo-random bytes."]
    #[doc = " "]
    #[doc = " This function must produce data at least as cryptographically secure and"]
    #[doc = " fast as an adequately seeded cryptographically-secure pseudo-random"]
    #[doc = " number generator (CSPRNG). It must not block, from the perspective of"]
    #[doc = " the calling program, under any circumstances, including on the first"]
    #[doc = " request and on requests for numbers of bytes. The returned data must"]
    #[doc = " always be unpredictable."]
    #[doc = " "]
    #[doc = " This function must always return fresh data. Deterministic environments"]
    #[doc = " must omit this function, rather than implementing it with deterministic"]
    #[doc = " data."]
    fn get_random_bytes(&mut self, len: u64) -> wasmtime::Result<Vec<u8>> {
        todo!()
    }

    #[doc = " Return a cryptographically-secure random or pseudo-random `u64` value."]
    #[doc = " "]
    #[doc = " This function returns the same type of data as `get-random-bytes`,"]
    #[doc = " represented as a `u64`."]
    fn get_random_u64(&mut self) -> wasmtime::Result<u64> {
        todo!()
    }
}
