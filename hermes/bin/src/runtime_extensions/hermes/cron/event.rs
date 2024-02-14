//! Cron runtime extension event handler implementation.

use crate::{
    event_queue::event::HermesEventPayload,
    runtime_extensions::bindings::hermes::cron::api::CronTagged,
};

///
struct OnCronEvent {
    ///
    tag: CronTagged,
    ///
    last: bool,
}

impl HermesEventPayload for OnCronEvent {
    fn event_name(&self) -> &str {
        "on-cron"
    }

    fn execute(&self, module: &mut crate::wasm::module::ModuleInstance) -> anyhow::Result<()> {
        module.instance.hermes_cron_event().call_on_cron(
            &mut module.store,
            &self.tag,
            self.last,
        )?;
        Ok(())
    }
}
