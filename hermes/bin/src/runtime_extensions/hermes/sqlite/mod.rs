//! `SQLite` runtime extension implementation.

use tracing::debug;

use crate::{
    app::ApplicationName,
    runtime_context::HermesRuntimeContext,
    runtime_extensions::{
        hermes::sqlite::{
            connection::core::close,
            state::{
                connection::DbHandle,
                resource_manager::{get_connection_pointer, init_app_state},
            },
        },
        init::{
            errors::RteInitResult, trait_app::RteInitApp, trait_event::RteInitEvent,
            trait_module::RteInitModule, trait_runtime::RteInitRuntime,
        },
    },
    wasm::module::ModuleId,
};

mod connection;
mod core;
mod host;
mod state;
mod statement;

/// Runtime Extension for `SQLite`
#[derive(Default)]
struct RteSqlite;

#[traitreg::register(default)]
impl RteInitRuntime for RteSqlite {}

#[traitreg::register(default)]
impl RteInitApp for RteSqlite {}

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

        for db_handle in [
            DbHandle::DiskRO,
            DbHandle::DiskRW,
            DbHandle::MemRO,
            DbHandle::MemRW,
        ] {
            if let Ok(db_ptr) = get_connection_pointer(ctx.app_name(), db_handle) {
                let _ = close(db_ptr as _);
            }
        }
        Ok(())
    }
}

/// Advise Runtime Extensions of a new context
pub(crate) fn new_context(_ctx: &crate::runtime_context::HermesRuntimeContext) {}
