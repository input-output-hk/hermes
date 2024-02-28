//! Hermes app implementation.

use crate::wasm::module::Module;

/// Hermes App Name type
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct HermesAppName(pub(crate) String);

/// Hermes app
#[allow(dead_code)]
pub(crate) struct HermesApp {
    /// App name
    app_name: HermesAppName,

    /// WASM modules
    modules: Vec<Module>,
}

impl HermesApp {
    /// Create a new Hermes app
    #[allow(dead_code)]
    pub(crate) fn new(app_name: HermesAppName, module_bytes: Vec<Vec<u8>>) -> anyhow::Result<Self> {
        let mut modules = Vec::with_capacity(module_bytes.len());
        for module_bytes in module_bytes {
            modules.push(Module::new(&module_bytes)?);
        }
        Ok(Self { app_name, modules })
    }

    /// Get app name
    #[allow(dead_code)]
    pub(crate) fn app_name(&self) -> &HermesAppName {
        &self.app_name
    }
}
