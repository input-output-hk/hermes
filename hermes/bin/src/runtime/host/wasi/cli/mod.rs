//! Host - WASI - CLI implementations
//!
#![allow(unused_variables)]

use crate::runtime::extensions::{
    wasi::{
        cli,
        io::streams::{InputStream, OutputStream},
    },
    HermesState, NewState,
};

/// WASI State
pub(crate) struct State {}

impl NewState for State {
    fn new(ctx: &crate::wasm::context::Context) -> Self {
        Self {}
    }
}

impl cli::environment::Host for HermesState {
    #[doc = " Get the POSIX-style environment variables."]
    #[doc = " "]
    #[doc = " Each environment variable is provided as a pair of string variable names"]
    #[doc = " and string value."]
    #[doc = " "]
    #[doc = " Morally, these are a value import, but until value imports are available"]
    #[doc = " in the component model, this import function should return the same"]
    #[doc = " values each time it is called."]
    fn get_environment(&mut self) -> wasmtime::Result<Vec<(String, String)>> {
        todo!()
    }

    #[doc = " Get the POSIX-style arguments to the program."]
    fn get_arguments(&mut self) -> wasmtime::Result<Vec<String>> {
        todo!()
    }

    #[doc = " Return a path that programs should use as their initial current working"]
    #[doc = " directory, interpreting `.` as shorthand for this."]
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
