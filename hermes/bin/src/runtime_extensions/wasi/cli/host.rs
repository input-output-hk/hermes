//! CLI host implementation for WASM runtime.

use tracing::warn;

use crate::{
    runtime_context::HermesRuntimeContext,
    runtime_extensions::{
        bindings::wasi::{
            cli,
            io::streams::{InputStream, OutputStream},
        },
        wasi::io::streams::{get_input_streams_state, get_output_streams_state},
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
        Ok(Vec::new())
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

impl cli::stdin::Host for HermesRuntimeContext {
    fn get_stdin(&mut self) -> wasmtime::Result<wasmtime::component::Resource<InputStream>> {
        warn!("Stdin is not supported");
        let app_state = get_input_streams_state().get_app_state(self.app_name())?;
        Ok(app_state.create_resource(Box::new(std::io::empty())))
    }
}

impl cli::stdout::Host for HermesRuntimeContext {
    fn get_stdout(&mut self) -> wasmtime::Result<wasmtime::component::Resource<OutputStream>> {
        // TODO: Redirect stdout to Hermes' logging api.
        let app_state = get_output_streams_state().get_app_state(self.app_name())?;
        Ok(app_state.create_resource(Box::new(std::io::empty())))
    }
}

impl cli::stderr::Host for HermesRuntimeContext {
    fn get_stderr(&mut self) -> wasmtime::Result<wasmtime::component::Resource<OutputStream>> {
        // TODO: Redirect stderr to Hermes' logging api.
        let app_state = get_output_streams_state().get_app_state(self.app_name())?;
        Ok(app_state.create_resource(Box::new(std::io::empty())))
    }
}
