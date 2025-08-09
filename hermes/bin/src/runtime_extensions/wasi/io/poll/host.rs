//! IO Poll host implementation for WASM runtime.

use crate::{
    runtime_context::HermesRuntimeContext,
    runtime_extensions::bindings::wasi::io::poll::{Host, HostPollable, Pollable},
};

// TODO: add support for `wasi:io/poll` (Issue <https://github.com/input-output-hk/hermes/issues/440>).
impl HostPollable for HermesRuntimeContext {
    /// Return the readiness of a pollable. This function never blocks.
    ///
    /// Returns `true` when the pollable is ready, and `false` otherwise.
    fn ready(
        &mut self,
        _self_: wasmtime::component::Resource<Pollable>,
    ) -> wasmtime::Result<bool> {
        #[allow(clippy::todo)]
        {
            todo!()
        }
    }

    /// `block` returns immediately if the pollable is ready, and otherwise
    /// blocks until ready.
    ///
    /// This function is equivalent to calling `poll.poll` on a list
    /// containing only this pollable.
    fn block(
        &mut self,
        _self_: wasmtime::component::Resource<Pollable>,
    ) -> wasmtime::Result<()> {
        #[allow(clippy::todo)]
        {
            todo!()
        }
    }

    fn drop(
        &mut self,
        _rep: wasmtime::component::Resource<Pollable>,
    ) -> wasmtime::Result<()> {
        #[allow(clippy::todo)]
        {
            todo!()
        }
    }
}

// TODO: add support for `wasi:io/poll` (Issue <https://github.com/input-output-hk/hermes/issues/440>).
impl Host for HermesRuntimeContext {
    /// Poll for completion on a set of pollables.
    ///
    /// This function takes a list of pollables, which identify I/O sources of
    /// interest, and waits until one or more of the events is ready for I/O.
    ///
    /// The result `list<u32>` contains one or more indices of handles in the
    /// argument list that is ready for I/O.
    ///
    /// This function traps if either:
    /// - the list is empty, or:
    /// - the list contains more elements than can be indexed with a `u32` value.
    ///
    /// A timeout can be implemented by adding a pollable from the
    /// wasi-clocks API to the list.
    ///
    /// This function does not return a `result`; polling in itself does not
    /// do any I/O so it doesn\'t fail. If any of the I/O sources identified by
    /// the pollables has an error, it is indicated by marking the source as
    /// being ready for I/O."]
    fn poll(
        &mut self,
        _in_: Vec<wasmtime::component::Resource<Pollable>>,
    ) -> wasmtime::Result<wasmtime::component::__internal::Vec<u32>> {
        #[allow(clippy::todo)]
        {
            todo!()
        }
    }
}
