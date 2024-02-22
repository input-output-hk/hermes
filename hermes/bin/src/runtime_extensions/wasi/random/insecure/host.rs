//! Insecure RNG host implementation for WASM runtime.

use crate::{runtime_extensions::bindings::wasi::random::insecure::Host, state::HermesState};

impl Host for HermesState {
    /// Return `len` insecure pseudo-random bytes.
    ///
    /// This function is not cryptographically secure. Do not use it for
    /// anything related to security.
    ///
    /// There are no requirements on the values of the returned bytes, however
    /// implementations are encouraged to return evenly distributed values with
    /// a long period.
    fn get_insecure_random_bytes(&mut self, _len: u64) -> wasmtime::Result<Vec<u8>> {
        todo!()
    }

    /// Return an insecure pseudo-random `u64` value.
    ///
    /// This function returns the same type of pseudo-random data as
    /// `get-insecure-random-bytes`, represented as a `u64`.
    fn get_insecure_random_u64(&mut self) -> wasmtime::Result<u64> {
        todo!()
    }
}
