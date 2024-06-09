//! HTTP-Gateway handler implementation.

use std::sync::mpsc::Sender;

use hyper::{self, body::Bytes};
use serde::{Deserialize, Serialize};

use crate::event::HermesEventPayload;

/// HTTP response code
type Code = u16;

/// Headers in kv form
type HeadersKV = Vec<(String, String)>;

/// HTTP Path
type Path = String;

/// HTTP method e.g GET
type Method = String;

/// HTTP raw headers bytes
type RawHeaders = Vec<u8>;

/// Req body
type Body = Vec<u8>;

#[derive(Serialize, Deserialize, Debug)]
pub enum HTTPEventMsg {
    HTTPEventReceiver,
    HttpEventResponse((Code, HeadersKV, Body)),
}

/// HTTP Event
pub struct HTTPEvent {
    pub(crate) headers: RawHeaders,
    pub(crate) method: Method,
    pub(crate) path: Path,
    pub(crate) body: Bytes,
    pub(crate) sender: Sender<HTTPEventMsg>,
}

impl HermesEventPayload for HTTPEvent {
    fn event_name(&self) -> &str {
        "http-event"
    }

    fn execute(&self, module: &mut crate::wasm::module::ModuleInstance) -> anyhow::Result<()> {
        let event_response = module.instance.hermes_http_gateway_event().call_reply(
            &mut module.store,
            &self.body.as_ref().to_vec(),
            &self.headers,
            &self.path,
            &self.method,
        )?;

        // http event will repsonse with none or response reply which i just respond and then flag
        // connection is done flag event is processed or not
        Ok(self.sender.send(HTTPEventMsg::HttpEventResponse((
            event_response.0,
            event_response.1,
            event_response.2,
        )))?)
    }
}
