//! Hermes app implementation.

use std::collections::HashMap;

use crate::wasm::module::{Module, ModuleId};

/// Hermes App Name type
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct HermesAppName(pub(crate) String);

impl std::fmt::Display for HermesAppName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

/// Convenient type alias for indexed apps map (`HermesAppName` -> `HermesApp`)
pub(crate) type IndexedApps = HashMap<HermesAppName, HermesApp>;

/// Hermes app
pub(crate) struct HermesApp {
    /// App name
    app_name: HermesAppName,

    /// WASM modules
    indexed_modules: HashMap<ModuleId, Module>,
}

impl HermesApp {
    /// Create a new Hermes app
    #[allow(dead_code)]
    pub(crate) fn new(app_name: HermesAppName, module_bytes: Vec<Vec<u8>>) -> anyhow::Result<Self> {
        let mut modules = HashMap::with_capacity(module_bytes.len());
        for module_bytes in module_bytes {
            let module = Module::new(&module_bytes)?;
            modules.insert(module.id().clone(), module);
        }
        Ok(Self {
            app_name,
            indexed_modules: modules,
        })
    }

    /// Get app name
    pub(crate) fn app_name(&self) -> &HermesAppName {
        &self.app_name
    }

    /// Get indexed modules
    pub(crate) fn indexed_modules(&self) -> &HashMap<ModuleId, Module> {
        &self.indexed_modules
    }
}
