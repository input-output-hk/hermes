pub trait HermesEventPayload<Executor> {
    /// Returns the name of the event associated with the payload.
    fn event_name(&self) -> &str;

    /// Executes the behavior associated with the payload, using the provided executor.
    ///
    /// # Arguments
    ///
    /// * `executor` - The  to use for executing the payload's behavior.
    ///
    /// # Returns
    ///
    /// An `anyhow::Result` indicating the success or failure of the payload execution.
    fn execute(&self, executor: &mut Executor) -> anyhow::Result<()>;
}
