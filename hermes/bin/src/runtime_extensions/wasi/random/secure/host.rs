//! Random RNG host implementation for WASM runtime.

use crate::{runtime_extensions::bindings::wasi::random::random::Host, state::HermesState};

impl Host for HermesState {
    /// Return `len` cryptographically-secure random or pseudo-random bytes.
    ///
    /// This function must produce data at least as cryptographically secure and
    /// fast as an adequately seeded cryptographically-secure pseudo-random
    /// number generator (Cryptographically Secure Random Number Generator).
    /// It must not block, from the perspective of
    /// the calling program, under any circumstances, including on the first
    /// request and on requests for numbers of bytes. The returned data must
    /// always be unpredictable.
    ///
    /// This function must always return fresh data. Deterministic environments
    /// must omit this function, rather than implementing it with deterministic
    /// data.
    fn get_random_bytes(&mut self, _len: u64) -> wasmtime::Result<Vec<u8>> {
        todo!()
    }

    /// Return a cryptographically-secure random or pseudo-random `u64` value.
    ///
    /// This function returns the same type of data as `get-random-bytes`,
    /// represented as a `u64`.
    fn get_random_u64(&mut self) -> wasmtime::Result<u64> {
        todo!()
    }
}
