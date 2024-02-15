//! CLI host implementation for WASM runtime.

use crate::{
    runtime_extensions::bindings::wasi::{
        cli,
        io::streams::{InputStream, OutputStream},
    },
    state::HermesState,
};

impl cli::environment::Host for HermesState {
    /// Get the POSIX-style environment variables.
    ///
    /// Each environment variable is provided as a pair of string variable names
    /// and string value.
    ///
    /// Morally, these are a value import, but until value imports are available
    /// in the component model, this import function should return the same
    /// values each time it is called.
    fn get_environment(&mut self) -> wasmtime::Result<Vec<(String, String)>> {
        todo!()
    }

    /// Get the POSIX-style arguments to the program.
    fn get_arguments(&mut self) -> wasmtime::Result<Vec<String>> {
        todo!()
    }

    /// Return a path that programs should use as their initial current working
    /// directory, interpreting `.` as shorthand for this.
    fn initial_cwd(&mut self) -> wasmtime::Result<Option<String>> {
        todo!()
    }
}

impl cli::stdin::Host for HermesState {
    fn get_stdin(&mut self) -> wasmtime::Result<wasmtime::component::Resource<InputStream>> {
        todo!()
    }
}

impl cli::stdout::Host for HermesState {
    fn get_stdout(&mut self) -> wasmtime::Result<wasmtime::component::Resource<OutputStream>> {
        todo!()
    }
}

impl cli::stderr::Host for HermesState {
    fn get_stderr(&mut self) -> wasmtime::Result<wasmtime::component::Resource<OutputStream>> {
        todo!()
    }
}
