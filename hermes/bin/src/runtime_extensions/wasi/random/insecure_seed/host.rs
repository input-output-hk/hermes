//! Insecure RNG seed host implementation for WASM runtime.

use crate::{runtime_extensions::bindings::wasi::random::insecure_seed::Host, state::HermesState};

impl Host for HermesState {
    /// Return a 128-bit value that may contain a pseudo-random value.
    ///
    /// The returned value is not required to be computed from a Cryptographically Secure
    /// RNG, and may
    /// even be entirely deterministic. Host implementations are encouraged to
    /// provide pseudo-random values to any program exposed to
    /// attacker-controlled content, to enable `DoS` protection built into many
    /// languages\' hash-map implementations.
    ///
    /// This function is intended to only be called once, by a source language
    /// to initialize Denial Of Service (`DoS`) protection in its hash-map
    /// implementation.
    ///
    /// # Expected future evolution
    ///
    /// This will likely be changed to a value import, to prevent it from being
    /// called multiple times and potentially used for purposes other than `DoS`
    /// protection.
    fn insecure_seed(&mut self) -> wasmtime::Result<(u64, u64)> {
        todo!()
    }
}
