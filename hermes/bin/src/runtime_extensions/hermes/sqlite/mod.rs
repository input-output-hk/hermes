//! `SQLite` runtime extension implementation.

use std::sync::Once;

use tracing::debug;

use crate::{
    app::ApplicationName,
    runtime_context::HermesRuntimeContext,
    runtime_extensions::{
        bindings::hermes::sqlite::api::Errno,
        hermes::sqlite::{
            connection::core::close_and_remove_all, kernel::open_with_persistent_memory,
            state::resource_manager::init_app_state,
        },
        init::{
            errors::{RteInitResult, RuntimeExtensionErrors},
            metadata::RteMetadata,
            trait_app::RteInitApp,
            trait_event::RteInitEvent,
            trait_module::RteInitModule,
            trait_runtime::RteInitRuntime,
        },
    },
    wasm::module::ModuleId,
};

mod connection;
mod host;
mod kernel;
mod state;
mod statement;

/// Controls [`is_parallel_event_execution`] value.
static SERIALIZED: Once = Once::new();

/// Disables parallel event execution.
pub(crate) fn set_serialized() {
    SERIALIZED.call_once(|| ());
}

/// Returns whether events are executed in parallel.
pub(crate) fn is_serialized() -> bool {
    SERIALIZED.is_completed()
}

/// Runtime Extension for `SQLite`
#[derive(Default)]
struct RteSqlite;

#[traitreg::register(default)]
impl RteInitRuntime for RteSqlite {}

#[traitreg::register(default)]
impl RteInitApp for RteSqlite {
    fn fini(
        self: Box<Self>,
        name: &ApplicationName,
    ) -> RteInitResult {
        debug!(%name,"Hermes Runtime Extensions Finalizing: App");

        // Cleaning up app-memory.
        match open_with_persistent_memory(true, true, name.clone()) {
            Ok(db_ptr) => close_and_remove_all(db_ptr),
            // App didn't have any in-memory connections – ok.
            // See <https://sqlite.org/rescode.html>.
            Err(Errno::Sqlite(14)) => Ok(()),
            Err(err) => Err(anyhow::Error::from(err)),
        }
        .map_err(|err| {
            let errors = RuntimeExtensionErrors::new();
            crate::add_rte_error!(errors, RteMetadata::none(), ImpossibleError {
                description: err.to_string()
            });
            errors
        })
    }
}

#[traitreg::register(default)]
impl RteInitModule for RteSqlite {
    fn init(
        self: Box<Self>,
        name: &ApplicationName,
        module_id: &ModuleId,
    ) -> RteInitResult {
        debug!(name=%name, module_id=%module_id,"Hermes Runtime Extensions Initialized: Module");
        init_app_state(name);

        Ok(())
    }
}

#[traitreg::register(default)]
impl RteInitEvent for RteSqlite {
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

        init_app_state(ctx.app_name());
        Ok(())
    }
}

/// Advise Runtime Extensions of a new context
pub(crate) fn new_context(_ctx: &crate::runtime_context::HermesRuntimeContext) {}
