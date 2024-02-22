//! Cron runtime extension event handler implementation.

use crate::{
    event_queue::event::HermesEventPayload,
    runtime_extensions::bindings::hermes::cron::api::CronTagged,
};

/// On cron event
struct OnCronEvent {
    /// The tagged cron event that was triggered.
    tag: CronTagged,
    /// This cron event will not retrigger.
    last: bool,
}

impl HermesEventPayload for OnCronEvent {
    fn event_name(&self) -> &str {
        "on-cron"
    }

    fn execute(&self, module: &mut crate::wasm::module::ModuleInstance) -> anyhow::Result<()> {
        // TODO (@stevenj): https://github.com/input-output-hk/hermes/issues/93
        let _res: bool = module.instance.hermes_cron_event().call_on_cron(
            &mut module.store,
            &self.tag,
            self.last,
        )?;
        Ok(())
    }
}
