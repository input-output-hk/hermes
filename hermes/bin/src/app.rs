//! Hermes app implementation.

use std::{collections::HashMap, sync::Arc};

use crate::{
    event::HermesEventPayload,
    pool,
    vfs::Vfs,
    wasm::module::{Module, ModuleId},
};

use tracing::info;

/// Hermes App Name type
#[cfg_attr(debug_assertions, derive(Debug))]
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct ApplicationName(pub(crate) String);

impl std::fmt::Display for ApplicationName {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl ApplicationName {
    /// Create a new `ApplicationName`.
    #[must_use]
    pub fn new(name: &str) -> Self {
        Self(name.to_string())
    }
}

/// Hermes application
pub(crate) struct Application {
    /// Application name
    name: ApplicationName,

    /// WASM modules
    indexed_modules: HashMap<ModuleId, Arc<Module>>,

    /// human modules
    human_modules: HashMap<String, ModuleId>,

    /// Application's `Vfs` instance
    vfs: Arc<Vfs>,
}

impl Application {
    /// Create a new Hermes app
    #[must_use]
    pub(crate) fn new(
        app_name: ApplicationName,
        vfs: Vfs,
        modules: Vec<Module>,
        human_modules: HashMap<String, ModuleId>,
    ) -> Self {
        let indexed_modules = modules
            .into_iter()
            .map(|module| (module.id().clone(), Arc::new(module)))
            .collect();
        Self {
            name: app_name,
            indexed_modules,
            vfs: Arc::new(vfs),
            human_modules,
        }
    }

    /// Get app name
    pub(crate) fn name(&self) -> &ApplicationName {
        &self.name
    }

    /// Get vfs
    pub(crate) fn vfs(&self) -> &Vfs {
        self.vfs.as_ref()
    }

    /// Get vfs
    pub(crate) fn get_human(&self) -> HashMap<std::string::String, ModuleId> {
        self.human_modules.clone()
    }

    /// Dispatch event for all available modules.
    pub(crate) fn dispatch_event(
        &self,
        event: &Arc<dyn HermesEventPayload>,
    ) {
        for module in self.indexed_modules.values() {
            module_dispatch_event(module.clone(), self.vfs.clone(), event.clone());
        }
    }

    /// Initialize every module.
    pub(crate) fn init(&self) -> anyhow::Result<()> {
        info!("Human modules ({} entries):", self.human_modules.len());

        for (name, module_id) in &self.human_modules {
            info!("  {} -> {}", name, module_id);
        }
        for module in self.indexed_modules.values() {
            if let Err(e) = module.init(self.vfs.clone()) {
                anyhow::bail!("Failed to initialize module {}: {}", module.id(), e)
            }
        }
        Ok(())
    }

    /// Dispatch event for the target module by the `module_id`.
    pub(crate) fn dispatch_event_for_target_module(
        &self,
        module_id: &ModuleId,
        event: Arc<dyn HermesEventPayload>,
    ) -> anyhow::Result<()> {
        let module = self
            .indexed_modules
            .get(module_id)
            .ok_or(anyhow::anyhow!("Module {module_id} not found"))?;
        module_dispatch_event(module.clone(), self.vfs.clone(), event);
        Ok(())
    }
}

/// Dispatch event
pub(crate) fn module_dispatch_event(
    module: Arc<Module>,
    vfs: Arc<Vfs>,
    event: Arc<dyn HermesEventPayload>,
) {
    // TODO(@aido-mth): fix how init is processed. https://github.com/input-output-hk/hermes/issues/490
    pool::execute(move || {
        if let Err(err) = module.execute_event(event.as_ref(), vfs) {
            tracing::error!("module event execution failed: {err}");
        }
    });
}
