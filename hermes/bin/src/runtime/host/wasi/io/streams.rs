//! WASI IO Streams
//!
#![allow(unused_variables)]

use crate::runtime::extensions::{
    wasi::io::streams::{
        Host, HostInputStream, HostOutputStream, InputStream, OutputStream, StreamError,
    },
    HermesState, NewState,
};

/// WASI State
pub(crate) struct State {}

impl NewState for State {
    fn new(_ctx: &crate::wasm::context::Context) -> Self {
        Self {}
    }
}

impl HostInputStream for HermesState {
    #[doc = " Perform a non-blocking read from the stream."]
    #[doc = " "]
    #[doc = " This function returns a list of bytes containing the read data,"]
    #[doc = " when successful. The returned list will contain up to `len` bytes;"]
    #[doc = " it may return fewer than requested, but not more. The list is"]
    #[doc = " empty when no bytes are available for reading at this time. The"]
    #[doc = " pollable given by `subscribe` will be ready when more bytes are"]
    #[doc = " available."]
    #[doc = " "]
    #[doc = " This function fails with a `stream-error` when the operation"]
    #[doc = " encounters an error, giving `last-operation-failed`, or when the"]
    #[doc = " stream is closed, giving `closed`."]
    #[doc = " "]
    #[doc = " When the caller gives a `len` of 0, it represents a request to"]
    #[doc = " read 0 bytes. If the stream is still open, this call should"]
    #[doc = " succeed and return an empty list, or otherwise fail with `closed`."]
    #[doc = " "]
    #[doc = " The `len` parameter is a `u64`, which could represent a list of u8 which"]
    #[doc = " is not possible to allocate in wasm32, or not desirable to allocate as"]
    #[doc = " as a return value by the callee. The callee may return a list of bytes"]
    #[doc = " less than `len` in size while more bytes are available for reading."]
    fn read(
        &mut self, self_: wasmtime::component::Resource<InputStream>, len: u64,
    ) -> wasmtime::Result<Result<Vec<u8>, StreamError>> {
        todo!()
    }

    #[doc = " Read bytes from a stream, after blocking until at least one byte can"]
    #[doc = " be read. Except for blocking, behavior is identical to `read`."]
    fn blocking_read(
        &mut self, self_: wasmtime::component::Resource<InputStream>, len: u64,
    ) -> wasmtime::Result<Result<Vec<u8>, StreamError>> {
        todo!()
    }

    #[doc = " Skip bytes from a stream. Returns number of bytes skipped."]
    #[doc = " "]
    #[doc = " Behaves identical to `read`, except instead of returning a list"]
    #[doc = " of bytes, returns the number of bytes consumed from the stream."]
    fn skip(
        &mut self, self_: wasmtime::component::Resource<InputStream>, len: u64,
    ) -> wasmtime::Result<Result<u64, StreamError>> {
        todo!()
    }

    #[doc = " Skip bytes from a stream, after blocking until at least one byte"]
    #[doc = " can be skipped. Except for blocking behavior, identical to `skip`."]
    fn blocking_skip(
        &mut self, self_: wasmtime::component::Resource<InputStream>, len: u64,
    ) -> wasmtime::Result<Result<u64, StreamError>> {
        todo!()
    }

    fn drop(&mut self, rep: wasmtime::component::Resource<InputStream>) -> wasmtime::Result<()> {
        todo!()
    }
}

impl HostOutputStream for HermesState {
    #[doc = " Check readiness for writing. This function never blocks."]
    #[doc = " "]
    #[doc = " Returns the number of bytes permitted for the next call to `write`,"]
    #[doc = " or an error. Calling `write` with more bytes than this function has"]
    #[doc = " permitted will trap."]
    #[doc = " "]
    #[doc = " When this function returns 0 bytes, the `subscribe` pollable will"]
    #[doc = " become ready when this function will report at least 1 byte, or an"]
    #[doc = " error."]
    fn check_write(
        &mut self, self_: wasmtime::component::Resource<OutputStream>,
    ) -> wasmtime::Result<Result<u64, StreamError>> {
        todo!()
    }

    #[doc = " Perform a write. This function never blocks."]
    #[doc = " "]
    #[doc = " Precondition: check-write gave permit of Ok(n) and contents has a"]
    #[doc = " length of less than or equal to n. Otherwise, this function will trap."]
    #[doc = " "]
    #[doc = " returns Err(closed) without writing if the stream has closed since"]
    #[doc = " the last call to check-write provided a permit."]
    fn write(
        &mut self, self_: wasmtime::component::Resource<OutputStream>, contents: Vec<u8>,
    ) -> wasmtime::Result<Result<(), StreamError>> {
        todo!()
    }

    #[doc = " Perform a write of up to 4096 bytes, and then flush the stream. Block"]
    #[doc = " until all of these operations are complete, or an error occurs."]
    #[doc = " "]
    #[doc = " This is a convenience wrapper around the use of `check-write`,"]
    #[doc = " `subscribe`, `write`, and `flush`, and is implemented with the"]
    #[doc = " following pseudo-code:"]
    #[doc = " "]
    #[doc = " ```text"]
    #[doc = " let pollable = this.subscribe();"]
    #[doc = " while !contents.is_empty() {"]
    #[doc = " // Wait for the stream to become writable"]
    #[doc = " poll-one(pollable);"]
    #[doc = " let Ok(n) = this.check-write(); // eliding error handling"]
    #[doc = " let len = min(n, contents.len());"]
    #[doc = " let (chunk, rest) = contents.split_at(len);"]
    #[doc = " this.write(chunk  );            // eliding error handling"]
    #[doc = " contents = rest;"]
    #[doc = " }"]
    #[doc = " this.flush();"]
    #[doc = " // Wait for completion of `flush`"]
    #[doc = " poll-one(pollable);"]
    #[doc = " // Check for any errors that arose during `flush`"]
    #[doc = " let _ = this.check-write();         // eliding error handling"]
    #[doc = " ```"]
    fn blocking_write_and_flush(
        &mut self, self_: wasmtime::component::Resource<OutputStream>, contents: Vec<u8>,
    ) -> wasmtime::Result<Result<(), StreamError>> {
        todo!()
    }

    #[doc = " Request to flush buffered output. This function never blocks."]
    #[doc = " "]
    #[doc = " This tells the output-stream that the caller intends any buffered"]
    #[doc = " output to be flushed. the output which is expected to be flushed"]
    #[doc = " is all that has been passed to `write` prior to this call."]
    #[doc = " "]
    #[doc = " Upon calling this function, the `output-stream` will not accept any"]
    #[doc = " writes (`check-write` will return `ok(0)`) until the flush has"]
    #[doc = " completed. The `subscribe` pollable will become ready when the"]
    #[doc = " flush has completed and the stream can accept more writes."]
    fn flush(
        &mut self, self_: wasmtime::component::Resource<OutputStream>,
    ) -> wasmtime::Result<Result<(), StreamError>> {
        todo!()
    }

    #[doc = " Request to flush buffered output, and block until flush completes"]
    #[doc = " and stream is ready for writing again."]
    fn blocking_flush(
        &mut self, self_: wasmtime::component::Resource<OutputStream>,
    ) -> wasmtime::Result<Result<(), StreamError>> {
        todo!()
    }

    #[doc = " *"]
    #[doc = "         /// Create a `pollable` which will resolve once the output-stream"]
    #[doc = "         /// is ready for more writing, or an error has occured. When this"]
    #[doc = "         /// pollable is ready, `check-write` will return `ok(n)` with n>0, or an"]
    #[doc = "         /// error."]
    #[doc = "         ///"]
    #[doc = "         /// If the stream is closed, this pollable is always ready immediately."]
    #[doc = "         ///"]
    #[doc = "         /// The created `pollable` is a child resource of the `output-stream`."]
    #[doc = "         /// Implementations may trap if the `output-stream` is dropped before"]
    #[doc = "         /// all derived `pollable`s created with this function are dropped."]
    #[doc = "         subscribe: func() -> pollable;"]
    #[doc = "         */"]
    #[doc = " Write zeroes to a stream."]
    #[doc = " "]
    #[doc = " this should be used precisely like `write` with the exact same"]
    #[doc = " preconditions (must use check-write first), but instead of"]
    #[doc = " passing a list of bytes, you simply pass the number of zero-bytes"]
    #[doc = " that should be written."]
    fn write_zeroes(
        &mut self, self_: wasmtime::component::Resource<OutputStream>, len: u64,
    ) -> wasmtime::Result<Result<(), StreamError>> {
        todo!()
    }

    #[doc = " Perform a write of up to 4096 zeroes, and then flush the stream."]
    #[doc = " Block until all of these operations are complete, or an error"]
    #[doc = " occurs."]
    #[doc = " "]
    #[doc = " This is a convenience wrapper around the use of `check-write`,"]
    #[doc = " `subscribe`, `write-zeroes`, and `flush`, and is implemented with"]
    #[doc = " the following pseudo-code:"]
    #[doc = " "]
    #[doc = " ```text"]
    #[doc = " let pollable = this.subscribe();"]
    #[doc = " while num_zeroes != 0 {"]
    #[doc = " // Wait for the stream to become writable"]
    #[doc = " poll-one(pollable);"]
    #[doc = " let Ok(n) = this.check-write(); // eliding error handling"]
    #[doc = " let len = min(n, num_zeroes);"]
    #[doc = " this.write-zeroes(len);         // eliding error handling"]
    #[doc = " num_zeroes -= len;"]
    #[doc = " }"]
    #[doc = " this.flush();"]
    #[doc = " // Wait for completion of `flush`"]
    #[doc = " poll-one(pollable);"]
    #[doc = " // Check for any errors that arose during `flush`"]
    #[doc = " let _ = this.check-write();         // eliding error handling"]
    #[doc = " ```"]
    fn blocking_write_zeroes_and_flush(
        &mut self, self_: wasmtime::component::Resource<OutputStream>, len: u64,
    ) -> wasmtime::Result<Result<(), StreamError>> {
        todo!()
    }

    #[doc = " Read from one stream and write to another."]
    #[doc = " "]
    #[doc = " The behavior of splice is equivelant to:"]
    #[doc = " 1. calling `check-write` on the `output-stream`"]
    #[doc = " 2. calling `read` on the `input-stream` with the smaller of the"]
    #[doc = " `check-write` permitted length and the `len` provided to `splice`"]
    #[doc = " 3. calling `write` on the `output-stream` with that read data."]
    #[doc = " "]
    #[doc = " Any error reported by the call to `check-write`, `read`, or"]
    #[doc = " `write` ends the splice and reports that error."]
    #[doc = " "]
    #[doc = " This function returns the number of bytes transferred; it may be less"]
    #[doc = " than `len`."]
    fn splice(
        &mut self, self_: wasmtime::component::Resource<OutputStream>,
        src: wasmtime::component::Resource<InputStream>, len: u64,
    ) -> wasmtime::Result<Result<u64, StreamError>> {
        todo!()
    }

    #[doc = " Read from one stream and write to another, with blocking."]
    #[doc = " "]
    #[doc = " This is similar to `splice`, except that it blocks until the"]
    #[doc = " `output-stream` is ready for writing, and the `input-stream`"]
    #[doc = " is ready for reading, before performing the `splice`."]
    fn blocking_splice(
        &mut self, self_: wasmtime::component::Resource<OutputStream>,
        src: wasmtime::component::Resource<InputStream>, len: u64,
    ) -> wasmtime::Result<Result<u64, StreamError>> {
        todo!()
    }

    fn drop(&mut self, rep: wasmtime::component::Resource<OutputStream>) -> wasmtime::Result<()> {
        todo!()
    }
}

impl Host for HermesState {}
