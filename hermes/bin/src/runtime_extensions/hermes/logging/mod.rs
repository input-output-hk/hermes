//! Logging runtime extension implementation.

use tracing::debug;

use crate::{
    app::ApplicationName,
    runtime_context::HermesRuntimeContext,
    runtime_extensions::init::{
        errors::RteInitResult, priority::RteInitPriority, trait_app::RteInitApp,
        trait_event::RteInitEvent, trait_module::RteInitModule, trait_runtime::RteInitRuntime,
    },
    wasm::module::ModuleId,
};

mod host;
mod log_msg;

/// Runtime Extension for Logging
#[derive(Default)]
struct RteLogging;

#[traitreg::register(default)]
impl RteInitRuntime for RteLogging {
    fn init(self: Box<Self>) -> RteInitResult {
        debug!("Hermes Runtime Extensions Initialized: Node Runtime");
        Ok(())
    }

    fn fini(self: Box<Self>) -> RteInitResult {
        debug!("Hermes Runtime Extensions Finalized: Node Runtime");
        Ok(())
    }

    fn priority(
        &self,
        init: bool,
    ) -> i32 {
        // Runs First on `init` and last on `fini`
        RteInitPriority {
            init: i32::MAX,
            fini: i32::MIN,
        }
        .priority(init)
    }
}

#[traitreg::register(default)]
impl RteInitApp for RteLogging {
    fn init(
        self: Box<Self>,
        name: &ApplicationName,
    ) -> RteInitResult {
        debug!(name=%name,"Hermes Runtime Extensions Initialized: App Runtime");
        Ok(())
    }

    fn fini(
        self: Box<Self>,
        name: &ApplicationName,
    ) -> RteInitResult {
        debug!(name=%name, "Hermes Runtime Extensions Finalized: App Runtime");
        Ok(())
    }

    fn priority(
        &self,
        init: bool,
    ) -> i32 {
        // Runs First on `init` and last on `fini`
        RteInitPriority {
            init: i32::MAX,
            fini: i32::MIN,
        }
        .priority(init)
    }
}

#[traitreg::register(default)]
impl RteInitModule for RteLogging {
    fn init(
        self: Box<Self>,
        name: &ApplicationName,
        module_id: &ModuleId,
    ) -> RteInitResult {
        debug!(name=%name, module_id=%module_id,"Hermes Runtime Extensions Initialized: Module");
        Ok(())
    }

    fn fini(
        self: Box<Self>,
        name: &ApplicationName,
        module_id: &ModuleId,
    ) -> RteInitResult {
        debug!(name=%name, module_id=%module_id, "Hermes Runtime Extensions Finalized: Module");
        Ok(())
    }

    fn priority(
        &self,
        init: bool,
    ) -> i32 {
        // Runs First on `init` and last on `fini`
        RteInitPriority {
            init: i32::MAX,
            fini: i32::MIN,
        }
        .priority(init)
    }
}

#[traitreg::register(default)]
impl RteInitEvent for RteLogging {
    fn init(
        self: Box<Self>,
        ctx: &HermesRuntimeContext,
    ) -> RteInitResult {
        debug!(
                name=%ctx.app_name(),
                module=%ctx.module_id(),
                event=%ctx.event_name(),
                exc_count=%ctx.exc_counter(),
                "Hermes Runtime Extensions Initialized: Event");
        Ok(())
    }

    fn fini(
        self: Box<Self>,
        ctx: &HermesRuntimeContext,
    ) -> RteInitResult {
        debug!(
                name=%ctx.app_name(),
                module=%ctx.module_id(),
                event=%ctx.event_name(),
                exc_count=%ctx.exc_counter(),
                "Hermes Runtime Extensions Finalized: Event");
        Ok(())
    }

    fn priority(
        &self,
        init: bool,
    ) -> i32 {
        // Runs First on `init` and last on `fini`
        RteInitPriority {
            init: i32::MAX,
            fini: i32::MIN,
        }
        .priority(init)
    }
}
