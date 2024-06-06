//! HTTP-Gateway handler implementation.

use std::sync::mpsc::Sender;

use crate::event::HermesEventPayload;
use hyper::{self, body::Bytes};
use tracing::info;

/// HTTP Event
pub struct HTTPEvent {
    pub(crate) headers: Vec<u8>,
    pub(crate) method: String,
    pub(crate) body: Bytes,
    pub(crate) sender: Sender<String>,
}

impl HermesEventPayload for HTTPEvent {
    fn event_name(&self) -> &str {
        "http-event"
    }

    fn execute(&self, module: &mut crate::wasm::module::ModuleInstance) -> anyhow::Result<()> {
        let value = module
            .instance
            .hermes_http_gateway_event()
            .call_reply(&mut module.store, &self.headers)?;

        info!("{:?} {:?}", self.body, self.method);

        info!("valz {:?}", value);

        // http wit thing which

        // valid hosts

        // http event will repsonse with none or response reply which i just respond and then flag connection is done
        // flag event is processed or not
        Ok(self.sender.send("should be ok".to_string())?)
    }
}
