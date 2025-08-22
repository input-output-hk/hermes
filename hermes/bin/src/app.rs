//! Hermes app implementation.

use std::{collections::HashMap, sync::Arc};

use crate::{
    event::HermesEventPayload,
    pool,
    runtime_context::HermesRuntimeContext,
    runtime_extensions::new_context,
    vfs::Vfs,
    wasm::module::{Module, ModuleId},
};

/// Hermes App Name type
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct ApplicationName(pub(crate) String);

impl std::fmt::Display for ApplicationName {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        self.0.fmt(f)
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
        app_name: String,
        vfs: Vfs,
        modules: Vec<Module>,
    ) -> Self {
        let indexed_modules = modules
            .into_iter()
            .map(|module| (module.id().clone(), Arc::new(module)))
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
    pub(crate) fn vfs(&self) -> &Vfs {
        self.vfs.as_ref()
    }

    /// Dispatch event for all available modules.
    pub(crate) fn dispatch_event(
        &self,
        event: &Arc<dyn HermesEventPayload>,
    ) {
        for module in self.indexed_modules.values() {
            module_dispatch_event(
                module.clone(),
                self.name.clone(),
                module.id().clone(),
                self.vfs.clone(),
                event.clone(),
            );
        }
    }

    /// Dispatch event for the target module by the `module_id`.
    pub(crate) fn dispatch_event_for_target_module(
        &self,
        module_id: ModuleId,
        event: Arc<dyn HermesEventPayload>,
    ) -> anyhow::Result<()> {
        let module = self
            .indexed_modules
            .get(&module_id)
            .ok_or(anyhow::anyhow!("Module {module_id} not found"))?;
        module_dispatch_event(
            module.clone(),
            self.name.clone(),
            module_id,
            self.vfs.clone(),
            event,
        );
        Ok(())
    }
}

/// Dispatch event
pub(crate) fn module_dispatch_event(
    module: Arc<Module>,
    app_name: ApplicationName,
    module_id: ModuleId,
    vfs: Arc<Vfs>,
    event: Arc<dyn HermesEventPayload>,
) {
    // TODO: fix how init is processed.
    pool::execute(move || {
        let runtime_ctx = HermesRuntimeContext::new(
            app_name,
            module_id,
            event.event_name().to_string(),
            module.exec_counter(),
            vfs,
        );

        // Advise Runtime Extensions of a new context
        new_context(&runtime_ctx);

        drop(
            module
                .execute_event(event.as_ref(), runtime_ctx)
                .inspect_err(|err| tracing::error!("module event execution failed: {err}")),
        );
    });
}
