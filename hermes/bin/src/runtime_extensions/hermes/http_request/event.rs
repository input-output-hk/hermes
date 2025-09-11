use crate::{event::HermesEventPayload, runtime_extensions::bindings::unchecked_exports};

unchecked_exports::define! {
    /// Extends [`wasmtime::component::Instance`] with guest functions for HTTP request.
    trait ComponentInstanceExt {
         #[wit("hermes:http-request/event", "on-http-response")]
        fn hermes_http_request_event_on_http_response(request_id: Option<u64>, response: &[u8]);
    }
}

/// Event payload for the `on-http-response` event.
pub(super) struct OnHttpResponseEvent {
    /// Optional request ID associated.
    pub(super) request_id: Option<u64>,
    /// Bytes representing the HTTP response.
    pub(super) response: Vec<u8>,
}

impl HermesEventPayload for OnHttpResponseEvent {
    fn event_name(&self) -> &'static str {
        "on-http-response"
    }

    fn execute(
        &self,
        module: &mut crate::wasm::module::ModuleInstance,
    ) -> anyhow::Result<()> {
        module.instance.hermes_http_request_event_on_http_response(
            &mut module.store,
            self.request_id,
            &self.response,
        )?;
        Ok(())
    }
}
