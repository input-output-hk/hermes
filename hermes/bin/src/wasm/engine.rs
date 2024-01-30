//! WASM engine implementation
//! Wrapper over the `wasmtime::Engine` struct with some specific configuration setup.

use std::ops::{Deref, DerefMut};

use wasmtime::{Config as WasmConfig, Engine as WasmEngine};

use super::Error;

/// WASM Engine struct
pub(crate) struct Engine(WasmEngine);

impl Deref for Engine {
    type Target = WasmEngine;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Engine {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Engine {
    /// Creates a new instance of the `Engine`.
    ///
    /// # Errors
    ///
    /// - `Error::Wasm`. Returns an error if the `WasmEngine` fails to initialize.
    #[allow(dead_code)]
    pub(crate) fn new() -> Result<Self, Error> {
        let mut config = WasmConfig::new();
        config.wasm_component_model(true);
        config.consume_fuel(false);

        let engine = WasmEngine::new(&config)?;

        Ok(Self(engine))
    }
}
