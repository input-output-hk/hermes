use crate::event::HermesEventPayload;

pub(super) struct OnHttpResponseEvent {
    pub(super) request_id: String,
    pub(super) response: String,
}

impl HermesEventPayload for OnHttpResponseEvent {
    fn event_name(&self) -> &'static str {
        "on-http-response"
    }

    fn execute(&self, module: &mut crate::wasm::module::ModuleInstance) -> anyhow::Result<()> {
        module
            .instance
            .hermes_http_request_event_on_http_response()
            .call_on_http_response(&mut module.store, &self.request_id, &self.response)?;
        Ok(())
    }
}
