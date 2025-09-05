//! Hermes event's primitives.

pub mod queue;

use std::sync::Arc;

use crate::{
    app::ApplicationName,
    wasm::module::{ModuleId, ModuleInstance},
};

/// A trait for defining the behavior of a Hermes event.
pub(crate) trait HermesEventPayload: Send + Sync + 'static {
    /// Returns the name of the event associated with the payload.
    fn event_name(&self) -> &'static str;

    /// Executes the behavior associated with the payload, using the provided executor.
    ///
    /// # Arguments
    ///
    /// * `executor` - The executor to use for executing the payload's behavior.
    ///
    /// # Returns
    ///
    /// An `anyhow::Result` indicating the success or failure of the payload execution.
    fn execute(
        &self,
        module: &mut ModuleInstance,
    ) -> anyhow::Result<()>;
}

/// Target Hermes app to execute the event
pub(crate) enum TargetApp {
    /// Execute for all available apps
    #[allow(dead_code)]
    All,
    /// Execute for a specific list of apps
    List(Vec<ApplicationName>),
}

/// Target WASM module to execute the event
pub(crate) enum TargetModule {
    /// Execute for all available modules
    All,
    /// Execute for a specific list of modules
    List(Vec<ModuleId>),
}

/// Hermes event
pub(crate) struct HermesEvent {
    /// The payload carried by the `HermesEvent`.
    payload: Arc<dyn HermesEventPayload>,

    /// Target app
    target_app: TargetApp,

    /// Target module
    target_module: TargetModule,
}

impl HermesEvent {
    /// Create a new Hermes event
    pub(crate) fn new(
        payload: impl HermesEventPayload,
        target_app: TargetApp,
        target_module: TargetModule,
    ) -> Self {
        Self {
            payload: Arc::new(payload),
            target_app,
            target_module,
        }
    }

    /// Get event's payload
    pub(crate) fn payload(&self) -> &Arc<dyn HermesEventPayload> {
        &self.payload
    }

    /// Get event's target app
    pub(crate) fn target_app(&self) -> &TargetApp {
        &self.target_app
    }

    /// Get event's target module
    pub(crate) fn target_module(&self) -> &TargetModule {
        &self.target_module
    }
}
