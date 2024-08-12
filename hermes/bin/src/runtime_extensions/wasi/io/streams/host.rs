//! IO Streams host implementation for WASM runtime.

use std::io::{Read, Seek, Write};

use crate::{
    runtime_context::HermesRuntimeContext,
    runtime_extensions::{
        bindings::wasi::io::streams::{
            Host, HostInputStream, HostOutputStream, InputStream, OutputStream, StreamError,
        },
        wasi::{
            descriptors::{Descriptor, NUL_REP, STDERR_REP, STDOUT_REP},
            state::STATE,
        },
    },
};

impl HostInputStream for HermesRuntimeContext {
    /// Perform a non-blocking read from the stream.
    ///
    /// This function returns a list of bytes containing the read data,
    /// when successful. The returned list will contain up to `len` bytes;
    /// it may return fewer than requested, but not more. The list is
    /// empty when no bytes are available for reading at this time. The
    /// pollable given by `subscribe` will be ready when more bytes are
    /// available.
    ///
    /// This function fails with a `stream-error` when the operation
    /// encounters an error, giving `last-operation-failed`, or when the
    /// stream is closed, giving `closed`.
    ///
    /// When the caller gives a `len` of 0, it represents a request to
    /// read 0 bytes. If the stream is still open, this call should
    /// succeed and return an empty list, or otherwise fail with `closed`.
    ///
    /// The `len` parameter is a `u64`, which could represent a list of u8 which
    /// is not possible to allocate in wasm32, or not desirable to allocate as
    /// as a return value by the callee. The callee may return a list of bytes
    /// less than `len` in size while more bytes are available for reading.
    fn read(
        &mut self, _rep: wasmtime::component::Resource<InputStream>, _len: u64,
    ) -> wasmtime::Result<Result<Vec<u8>, StreamError>> {
        todo!()
    }

    /// Read bytes from a stream, after blocking until at least one byte can
    /// be read. Except for blocking, behavior is identical to `read`.
    fn blocking_read(
        &mut self, resource: wasmtime::component::Resource<InputStream>, len: u64,
    ) -> wasmtime::Result<Result<Vec<u8>, StreamError>> {
        // NUL_REP always returns 0 bytes.
        if resource.rep() == NUL_REP {
            return Ok(Ok(Vec::new()));
        }

        let seek_start = match STATE
            .get_mut(self.app_name())
            .input_stream_mut(resource.rep())
        {
            Some(input_stream) => input_stream.at(),
            None => return Ok(Err(StreamError::Closed)),
        };

        let buf = match STATE
            .get_mut(self.app_name())
            .descriptor_mut(resource.rep())
        {
            Some(fd) => {
                match fd {
                    Descriptor::File(f) => {
                        let Ok(f_size) = f.size().and_then(|size| {
                            TryInto::<u64>::try_into(size).map_err(|e| anyhow::anyhow!(e))
                        }) else {
                            // TODO: Probably should return LastOperationFailed here
                            return Ok(Err(StreamError::Closed));
                        };

                        if f_size == 0 || seek_start >= f_size {
                            return Ok(Ok(Vec::new()));
                        }

                        // At this point seek_start < f_size, so subtracting is safe
                        let Ok(read_len) = len.min(f_size - seek_start).try_into() else {
                            return Ok(Err(StreamError::Closed));
                        };

                        let mut buf = vec![0u8; read_len];

                        if f.seek(std::io::SeekFrom::Start(seek_start)).is_err() {
                            return Ok(Err(StreamError::Closed));
                        }

                        if f.read(&mut buf).is_err() {
                            return Ok(Err(StreamError::Closed));
                        }

                        buf
                    },
                    Descriptor::Dir(_) => todo!(),
                }
            },
            None => {
                return Ok(Err(StreamError::Closed));
            },
        };

        match STATE
            .get_mut(self.app_name())
            .input_stream_mut(resource.rep())
        {
            Some(input_stream) => input_stream.advance(buf.len() as u64),
            None => return Ok(Err(StreamError::Closed)),
        }

        Ok(Ok(buf))
    }

    /// Skip bytes from a stream. Returns number of bytes skipped.
    ///
    /// Behaves identical to `read`, except instead of returning a list
    /// of bytes, returns the number of bytes consumed from the stream.
    fn skip(
        &mut self, _rep: wasmtime::component::Resource<InputStream>, _len: u64,
    ) -> wasmtime::Result<Result<u64, StreamError>> {
        todo!()
    }

    /// Skip bytes from a stream, after blocking until at least one byte
    /// can be skipped. Except for blocking behavior, identical to `skip`.
    fn blocking_skip(
        &mut self, rep: wasmtime::component::Resource<InputStream>, len: u64,
    ) -> wasmtime::Result<Result<u64, StreamError>> {
        let skipped = match self.blocking_read(rep, len)? {
            Ok(buf) => buf.len(),
            Err(e) => return Ok(Err(e)),
        };

        Ok(Ok(skipped as u64))
    }

    fn drop(&mut self, rep: wasmtime::component::Resource<InputStream>) -> wasmtime::Result<()> {
        STATE
            .get_mut(self.app_name())
            .remove_input_stream(rep.rep());
        Ok(())
    }
}

impl HostOutputStream for HermesRuntimeContext {
    /// Check readiness for writing. This function never blocks.
    ///
    /// Returns the number of bytes permitted for the next call to `write`,
    /// or an error. Calling `write` with more bytes than this function has
    /// permitted will trap.
    ///
    /// When this function returns 0 bytes, the `subscribe` pollable will
    /// become ready when this function will report at least 1 byte, or an
    /// error.
    fn check_write(
        &mut self, _rep: wasmtime::component::Resource<OutputStream>,
    ) -> wasmtime::Result<Result<u64, StreamError>> {
        /// The number of bytes permitted for the next call to `write`.
        const PERMITTED_BYTES_TO_WRITE_NUM: u64 = 1024 * 1024;
        Ok(Ok(PERMITTED_BYTES_TO_WRITE_NUM))
    }

    /// Perform a write. This function never blocks.
    ///
    /// Precondition: check-write gave permit of Ok(n) and contents has a
    /// length of less than or equal to n. Otherwise, this function will trap.
    ///
    /// returns Err(closed) without writing if the stream has closed since
    /// the last call to check-write provided a permit.
    fn write(
        &mut self, _rep: wasmtime::component::Resource<OutputStream>, _contents: Vec<u8>,
    ) -> wasmtime::Result<Result<(), StreamError>> {
        todo!()
    }

    /// Perform a write of up to 4096 bytes, and then flush the stream. Block
    /// until all of these operations are complete, or an error occurs.
    ///
    /// This is a convenience wrapper around the use of `check-write`,
    /// `subscribe`, `write`, and `flush`, and is implemented with the
    /// following pseudo-code:
    ///
    /// ```text
    /// let pollable = this.subscribe();
    /// while !contents.is_empty() {
    /// // Wait for the stream to become writable
    /// poll-one(pollable);
    /// let Ok(n) = this.check-write(); // eliding error handling
    /// let len = min(n, contents.len());
    /// let (chunk, rest) = contents.split_at(len);
    /// this.write(chunk  );            // eliding error handling
    /// contents = rest;
    /// }
    /// this.flush();
    /// // Wait for completion of `flush`
    /// poll-one(pollable);
    /// // Check for any errors that arose during `flush`
    /// let _ = this.check-write();         // eliding error handling
    /// ```
    fn blocking_write_and_flush(
        &mut self, rep: wasmtime::component::Resource<OutputStream>, contents: Vec<u8>,
    ) -> wasmtime::Result<Result<(), StreamError>> {
        match rep.rep() {
            NUL_REP => {
                // Discard all bytes.
                return Ok(Ok(()));
            },
            STDOUT_REP => {
                if std::io::stdout().write_all(&contents).is_err() {
                    return Ok(Err(StreamError::Closed));
                }

                if std::io::stdout().flush().is_err() {
                    return Ok(Err(StreamError::Closed));
                }

                return Ok(Ok(()));
            },
            STDERR_REP => {
                if std::io::stderr().write_all(&contents).is_err() {
                    return Ok(Err(StreamError::Closed));
                }

                if std::io::stderr().flush().is_err() {
                    return Ok(Err(StreamError::Closed));
                }

                return Ok(Ok(()));
            },
            _ => {},
        }

        let seek_start = match STATE.get_mut(self.app_name()).output_stream_mut(rep.rep()) {
            Some(output_stream) => output_stream.at(),
            None => return Ok(Err(StreamError::Closed)),
        };

        match STATE.get_mut(self.app_name()).descriptor_mut(rep.rep()) {
            Some(fd) => {
                match fd {
                    // TODO: I believe it's better to return a LastOperationFailed error if
                    // any write operations fail.
                    Descriptor::File(f) => {
                        if f.seek(std::io::SeekFrom::Start(seek_start)).is_err() {
                            return Ok(Err(StreamError::Closed));
                        }

                        if f.write_all(&contents).is_err() {
                            return Ok(Err(StreamError::Closed));
                        }

                        if f.flush().is_err() {
                            return Ok(Err(StreamError::Closed));
                        }
                    },
                    Descriptor::Dir(_) => return Ok(Err(StreamError::Closed)),
                }
            },
            None => return Ok(Err(StreamError::Closed)),
        };

        match STATE.get_mut(self.app_name()).output_stream_mut(rep.rep()) {
            Some(output_stream) => {
                match contents.len().try_into() {
                    Ok(len) => output_stream.advance(len),
                    Err(_) => return Ok(Err(StreamError::Closed)),
                }
            },
            None => return Ok(Err(StreamError::Closed)),
        }

        Ok(Ok(()))
    }

    /// Request to flush buffered output. This function never blocks.
    ///
    /// This tells the output-stream that the caller intends any buffered
    /// output to be flushed. the output which is expected to be flushed
    /// is all that has been passed to `write` prior to this call.
    ///
    /// Upon calling this function, the `output-stream` will not accept any
    /// writes (`check-write` will return `ok(0)`) until the flush has
    /// completed. The `subscribe` pollable will become ready when the
    /// flush has completed and the stream can accept more writes.
    fn flush(
        &mut self, _rep: wasmtime::component::Resource<OutputStream>,
    ) -> wasmtime::Result<Result<(), StreamError>> {
        todo!()
    }

    /// Request to flush buffered output, and block until flush completes
    /// and stream is ready for writing again.
    fn blocking_flush(
        &mut self, _rep: wasmtime::component::Resource<OutputStream>,
    ) -> wasmtime::Result<Result<(), StreamError>> {
        todo!()
    }

    /// Write zeroes to a stream.
    ///
    /// this should be used precisely like `write` with the exact same
    /// preconditions (must use check-write first), but instead of
    /// passing a list of bytes, you simply pass the number of zero-bytes
    /// that should be written.
    fn write_zeroes(
        &mut self, _rep: wasmtime::component::Resource<OutputStream>, _len: u64,
    ) -> wasmtime::Result<Result<(), StreamError>> {
        todo!()
    }

    /// Perform a write of up to 4096 zeroes, and then flush the stream.
    /// Block until all of these operations are complete, or an error
    /// occurs.
    ///
    /// This is a convenience wrapper around the use of `check-write`,
    /// `subscribe`, `write-zeroes`, and `flush`, and is implemented with
    /// the following pseudo-code:
    ///
    /// ```text
    /// let pollable = this.subscribe();
    /// while num_zeroes != 0 {
    /// // Wait for the stream to become writable
    /// poll-one(pollable);
    /// let Ok(n) = this.check-write(); // eliding error handling
    /// let len = min(n, num_zeroes);
    /// this.write-zeroes(len);         // eliding error handling
    /// num_zeroes -= len;
    /// }
    /// this.flush();
    /// // Wait for completion of `flush`
    /// poll-one(pollable);
    /// // Check for any errors that arose during `flush`
    /// let _ = this.check-write();         // eliding error handling
    /// ```
    fn blocking_write_zeroes_and_flush(
        &mut self, _rep: wasmtime::component::Resource<OutputStream>, _len: u64,
    ) -> wasmtime::Result<Result<(), StreamError>> {
        todo!()
    }

    /// Read from one stream and write to another.
    ///
    /// The behavior of splice is equivalent to:
    /// 1. calling `check-write` on the `output-stream`
    /// 2. calling `read` on the `input-stream` with the smaller of the
    /// `check-write` permitted length and the `len` provided to `splice`
    /// 3. calling `write` on the `output-stream` with that read data.
    ///
    /// Any error reported by the call to `check-write`, `read`, or
    /// `write` ends the splice and reports that error.
    ///
    /// This function returns the number of bytes transferred; it may be less
    /// than `len`.
    fn splice(
        &mut self, _rep: wasmtime::component::Resource<OutputStream>,
        _src: wasmtime::component::Resource<InputStream>, _len: u64,
    ) -> wasmtime::Result<Result<u64, StreamError>> {
        todo!()
    }

    /// Read from one stream and write to another, with blocking.
    ///
    /// This is similar to `splice`, except that it blocks until the
    /// `output-stream` is ready for writing, and the `input-stream`
    /// is ready for reading, before performing the `splice`.
    fn blocking_splice(
        &mut self, _rep: wasmtime::component::Resource<OutputStream>,
        _src: wasmtime::component::Resource<InputStream>, _len: u64,
    ) -> wasmtime::Result<Result<u64, StreamError>> {
        todo!()
    }

    fn drop(&mut self, rep: wasmtime::component::Resource<OutputStream>) -> wasmtime::Result<()> {
        STATE
            .get_mut(self.app_name())
            .remove_output_stream(rep.rep());
        Ok(())
    }
}

impl Host for HermesRuntimeContext {}
