//! Hermes app implementation.

use std::{collections::HashMap, sync::Arc};

use crate::{
    event::HermesEventPayload,
    runtime_context::HermesRuntimeContext,
    runtime_extensions::new_context,
    vfs::Vfs,
    wasm::module::{Module, ModuleId},
};

/// Hermes App Name type
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct ApplicationName(pub(crate) String);

impl std::fmt::Display for ApplicationName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

/// Hermes application
pub(crate) struct Application {
    /// Application name
    name: ApplicationName,

    /// WASM modules
    indexed_modules: HashMap<ModuleId, Module>,

    /// Application's `Vfs` instance
    vfs: Arc<Vfs>,
}

impl Application {
    /// Create a new Hermes app
    pub(crate) fn new(app_name: String, vfs: Vfs, modules: Vec<Module>) -> Self {
        let indexed_modules = modules
            .into_iter()
            .map(|module| (module.id().clone(), module))
            .collect();
        Self {
            name: ApplicationName(app_name),
            indexed_modules,
            vfs: Arc::new(vfs),
        }
    }

    /// Get app name
    pub(crate) fn name(&self) -> &ApplicationName {
        &self.name
    }

    /// Get vfs
    #[allow(dead_code)]
    pub(crate) fn vfs(&self) -> &Vfs {
        self.vfs.as_ref()
    }

    /// Dispatch event for all available modules.
    pub(crate) fn dispatch_event(&self, event: &dyn HermesEventPayload) -> anyhow::Result<()> {
        for module in self.indexed_modules.values() {
            module_dispatch_event(
                module,
                self.name.clone(),
                module.id().clone(),
                self.vfs.clone(),
                event,
            )?;
        }
        Ok(())
    }

    /// Dispatch event for the target module by the `module_id`.
    pub(crate) fn dispatch_event_for_target_module(
        &self, module_id: ModuleId, event: &dyn HermesEventPayload,
    ) -> anyhow::Result<()> {
        let module = self
            .indexed_modules
            .get(&module_id)
            .ok_or(anyhow::anyhow!("Module {module_id} not found"))?;
        module_dispatch_event(
            module,
            self.name.clone(),
            module_id,
            self.vfs.clone(),
            event,
        )
    }
}

/// Dispatch event
pub(crate) fn module_dispatch_event(
    module: &Module, app_name: ApplicationName, module_id: ModuleId, vfs: Arc<Vfs>,
    event: &dyn HermesEventPayload,
) -> anyhow::Result<()> {
    let runtime_ctx = HermesRuntimeContext::new(
        app_name,
        module_id,
        event.event_name().to_string(),
        module.exec_counter(),
        vfs,
    );

    // Advise Runtime Extensions of a new context
    new_context(&runtime_ctx);

    module.execute_event(event, runtime_ctx)?;
    Ok(())
}
