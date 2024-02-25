//! Hermes event's primitives.

pub(crate) mod event_queue;
pub(crate) mod exec_manager;

use crate::{
    app::HermesAppName,
    wasm::module::{ModuleId, ModuleInstance},
};

/// A trait for defining the behavior of a Hermes event.
pub(crate) trait HermesEventPayload: Send + Sync + 'static {
    /// Returns the name of the event associated with the payload.
    fn event_name(&self) -> &str;

    /// Executes the behavior associated with the payload, using the provided executor.
    ///
    /// # Arguments
    ///
    /// * `executor` - The executor to use for executing the payload's behavior.
    ///
    /// # Returns
    ///
    /// An `anyhow::Result` indicating the success or failure of the payload execution.
    fn execute(&self, module: &mut ModuleInstance) -> anyhow::Result<()>;
}

/// Target Hermes app to execute the event
pub(crate) enum TargetApp {
    /// Execute for all available apps
    All,
    /// Execute for a specific list of apps
    _List(Vec<HermesAppName>),
}

/// Target WASM module to execute the event
pub(crate) enum TargetModule {
    /// Execute for all available modules
    All,
    /// Execute for a specific list of modules
    _List(Vec<ModuleId>),
}

/// Hermes event
pub(crate) struct HermesEvent {
    /// The payload carried by the HermesEvent.
    payload: Box<dyn HermesEventPayload>,

    /// Target app
    target_app: TargetApp,

    /// Target module
    target_module: TargetModule,
}

impl HermesEvent {
    /// Create a new Hermes event
    pub(crate) fn new(
        payload: impl HermesEventPayload, target_app: TargetApp, target_module: TargetModule,
    ) -> Self {
        Self {
            payload: Box::new(payload),
            target_app,
            target_module,
        }
    }

    /// Get event's payload
    pub(crate) fn payload(&self) -> &dyn HermesEventPayload {
        self.payload.as_ref()
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
