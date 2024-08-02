//! CLI host implementation for WASM runtime.

use tracing::warn;

use crate::{
    runtime_context::HermesRuntimeContext,
    runtime_extensions::{
        bindings::wasi::{
            cli,
            io::streams::{InputStream, OutputStream},
        },
        wasi::descriptors::{NUL_REP, STDERR_REP, STDOUT_REP},
    },
};

impl cli::environment::Host for HermesRuntimeContext {
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

impl cli::exit::Host for HermesRuntimeContext {
    fn exit(&mut self, _status: Result<(), ()>) -> wasmtime::Result<()> {
        warn!("Exiting an application is not supported");
        Ok(())
    }
}

impl cli::stdin::Host for HermesRuntimeContext {
    fn get_stdin(&mut self) -> wasmtime::Result<wasmtime::component::Resource<InputStream>> {
        warn!("Stdin is not supported");
        Ok(wasmtime::component::Resource::new_own(NUL_REP))
    }
}

impl cli::stdout::Host for HermesRuntimeContext {
    fn get_stdout(&mut self) -> wasmtime::Result<wasmtime::component::Resource<OutputStream>> {
        // TODO: Redirect stdout to Hermes' logging api.
        Ok(wasmtime::component::Resource::new_own(STDOUT_REP))
    }
}

impl cli::stderr::Host for HermesRuntimeContext {
    fn get_stderr(&mut self) -> wasmtime::Result<wasmtime::component::Resource<OutputStream>> {
        // TODO: Redirect stderr to Hermes' logging api.
        Ok(wasmtime::component::Resource::new_own(STDERR_REP))
    }
}
