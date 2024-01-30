use std::ops::{Deref, DerefMut};

use wasmtime::{Config as WasmConfig, Engine as WasmEngine};

use super::Error;

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
    pub(crate) fn new() -> Result<Self, Error> {
        let mut config = WasmConfig::new();
        config.wasm_component_model(true);
        config.consume_fuel(false);

        let engine = WasmEngine::new(&config)?;

        Ok(Self(engine))
    }
}
