//! Hermes app implementation.

use std::{collections::HashMap, path::Path};

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

    /// Loads app from directory
    pub(crate) fn from_dir(
        app_name: HermesAppName, path: impl AsRef<Path>,
    ) -> anyhow::Result<Self> {
        let entries = std::fs::read_dir(path)?;
        let mut wasm_modules_bytes = Vec::new();
        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() && path.extension() == Some(std::ffi::OsStr::new("wasm")) {
                let module_bytes = std::fs::read(path)?;
                wasm_modules_bytes.push(module_bytes);
            }
        }

        Self::new(app_name, wasm_modules_bytes)
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
