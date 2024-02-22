//! Hermes event definition

use crate::wasm::module::ModuleInstance;

/// A trait for defining the behavior of a Hermes event.
pub trait HermesEventPayload: Send {
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
