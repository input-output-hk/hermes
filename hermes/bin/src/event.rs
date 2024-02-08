pub trait HermesEventPayload<ModuleInstance> {
    /// Returns the name of the event associated with the payload.
    fn event_name(&self) -> &str;

    /// Executes the behavior associated with the payload, using the provided Hermes
    /// bindings and state store.
    ///
    /// # Arguments
    ///
    /// * `instance` - The Hermes instance to use for executing the payload's behavior.
    ///
    /// # Returns
    ///
    /// An `anyhow::Result` indicating the success or failure of the payload execution.
    fn execute(&self, instance: &mut ModuleInstance) -> anyhow::Result<()>;
}
