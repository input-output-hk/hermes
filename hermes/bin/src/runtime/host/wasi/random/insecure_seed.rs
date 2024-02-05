//! Insecure RNG

use crate::runtime::extensions::{wasi::random::insecure_seed::Host, HermesState, NewState};

/// WASI State
pub(crate) struct State {}

impl NewState for State {
    fn new(_ctx: &crate::wasm::context::Context) -> Self {
        Self {}
    }
}
impl Host for HermesState {
    #[doc = " Return a 128-bit value that may contain a pseudo-random value."]
    #[doc = " "]
    #[doc = " The returned value is not required to be computed from a Cryptographically Secure RNG, and may"]
    #[doc = " even be entirely deterministic. Host implementations are encouraged to"]
    #[doc = " provide pseudo-random values to any program exposed to"]
    #[doc = " attacker-controlled content, to enable DoS protection built into many"]
    #[doc = " languages\\' hash-map implementations."]
    #[doc = " "]
    #[doc = " This function is intended to only be called once, by a source language"]
    #[doc = " to initialize Denial Of Service (DoS) protection in its hash-map"]
    #[doc = " implementation."]
    #[doc = " "]
    #[doc = " # Expected future evolution"]
    #[doc = " "]
    #[doc = " This will likely be changed to a value import, to prevent it from being"]
    #[doc = " called multiple times and potentially used for purposes other than DoS"]
    #[doc = " protection."]
    fn insecure_seed(&mut self) -> wasmtime::Result<(u64, u64)> {
        todo!()
    }
}
