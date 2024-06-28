//! HTTP-Gateway handler implementation.

use crossbeam_channel::Sender;
use hyper::{self, body::Bytes};
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::event::HermesEventPayload;

/// HTTP response code
type Code = u16;

/// Headers in kv form
pub type HeadersKV = Vec<(String, Vec<String>)>;

/// HTTP Path
type Path = String;

/// HTTP method e.g GET
type Method = String;

/// Req body
type Body = Vec<u8>;

#[derive(Serialize, Deserialize, Debug)]
pub(crate) enum HTTPEventMsg {
    HTTPEventReceiver,
    HttpEventResponseSome((Code, HeadersKV, Body)),
    HttpEventResponseNone(),
}

/// HTTP Event
#[derive(Clone)]
pub(crate) struct HTTPEvent {
    pub(crate) headers: HeadersKV,
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

        info!(
            "Module propagation instance {:?}",
            rusty_ulid::generate_ulid_string()
        );

        if let Some(resp) = event_response {
            Ok(self.sender.send(HTTPEventMsg::HttpEventResponseSome((
                resp.code,
                resp.headers,
                resp.body,
            )))?)
        } else {
            Ok(self.sender.send(HTTPEventMsg::HttpEventResponseNone())?)
        }
    }
}
