//! WASM engine implementation
//! Wrapper over the `wasmtime::Engine` struct with some specific configuration setup.

use std::ops::{Deref, DerefMut};

use anyhow::anyhow;
use wasmtime::{Config as WasmConfig, Engine as WasmEngine};

/// WASM Engine struct
#[derive(Clone)]
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
    ///  - `BadEngineConfigError`
    pub(crate) fn new() -> anyhow::Result<Self> {
        let mut config = WasmConfig::new();
        config.wasm_component_model(true);
        config.consume_fuel(false);

        let engine = WasmEngine::new(&config).map_err(|e| {
            anyhow!(
                "Incorrect `wasmtime::Engine` configuration: {}",
                e.to_string()
            )
        })?;

        Ok(Self(engine))
    }
}
