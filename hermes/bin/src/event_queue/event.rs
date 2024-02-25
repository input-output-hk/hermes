//! Hermes event definition

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
    _payload: Box<dyn HermesEventPayload>,

    /// Target app
    _target_app: TargetApp,

    /// Target module
    _target_module: TargetModule,
}

impl HermesEvent {
    /// Create a new Hermes event
    pub(crate) fn new(
        payload: impl HermesEventPayload, target_app: TargetApp, target_module: TargetModule,
    ) -> Self {
        Self {
            _payload: Box::new(payload),
            _target_app: target_app,
            _target_module: target_module,
        }
    }
}
