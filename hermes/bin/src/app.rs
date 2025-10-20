//! Hermes app implementation.

use std::{collections::HashMap, sync::Arc};

use crate::{
    event::HermesEventPayload,
    pool,
    runtime_extensions::init::trait_app::{RteApp, RteInitApp as _},
    vfs::Vfs,
    wasm::module::{Module, ModuleId},
};

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
    ) -> Self {
        let indexed_modules = modules
            .into_iter()
            .map(|module| (module.id().clone(), Arc::new(module)))
            .collect();
        Self {
            name: app_name,
            indexed_modules,
            vfs: Arc::new(vfs),
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
        for module in self.indexed_modules.values() {
            if let Err(e) = module.init(self.vfs.clone()) {
                anyhow::bail!("Failed to initialize module {}: {:#}", module.id(), e)
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

impl Drop for Application {
    fn drop(&mut self) {
        // Advise Runtime Extensions that application is fully stopped and its resources can be
        // freed.
        if let Err(error) = RteApp::new().fini(&self.name) {
            tracing::error!(name = %self.name, %error, "Application failed to finalize");
        } else {
            tracing::info!(name = %self.name, "Application finalized successfully");
        }
    }
}

/// Dispatch event
pub(crate) fn module_dispatch_event(
    module: Arc<Module>,
    vfs: Arc<Vfs>,
    event: Arc<dyn HermesEventPayload>,
) {
    pool::execute(move || {
        if let Err(err) = module.execute_event(event.as_ref(), vfs) {
            tracing::error!("module event execution failed: {err}");
        }
    });
}
