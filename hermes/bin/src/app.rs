//! Hermes app implementation.

use crate::wasm::module::Module;

/// Hermes app
#[allow(dead_code)]
pub(crate) struct HermesApp {
    /// App name
    app_name: String,

    /// WASM modules
    wasm_modules: Vec<Module>,
}

impl HermesApp {
    /// Create a new Hermes app
    pub(crate) fn new(app_name: String, module_bytes: Vec<Vec<u8>>) -> anyhow::Result<Self> {
        let mut wasm_modules = Vec::with_capacity(module_bytes.len());
        for module_bytes in module_bytes {
            wasm_modules.push(Module::new(&module_bytes)?);
        }
        Ok(Self {
            app_name,
            wasm_modules,
        })
    }
}
