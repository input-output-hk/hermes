use std::{
    net::SocketAddr,
    sync::{mpsc::channel, Arc},
};

use anyhow::{anyhow, Ok};
use hyper::{self, Body, HeaderMap, Method, Request, Response, StatusCode};
use serde::Deserialize;
use tracing::info;

use super::gateway_task::ConnectionManager;
use crate::{
    event::{HermesEvent, TargetApp, TargetModule},
    runtime_extensions::hermes::kv_store::event::KVGet,
};

#[derive(Deserialize, Debug)]
pub struct Event {
    _body: String,
}

#[derive(Debug)]
pub struct Hostname(pub String);

pub fn _error_response(err: String) -> Response<Body> {
    Response::builder()
        .status(StatusCode::INTERNAL_SERVER_ERROR)
        .body(err.into())
        .unwrap()
}

pub fn _not_found() -> Response<Body> {
    Response::builder()
        .status(StatusCode::NOT_FOUND)
        .body("Not Found".into())
        .unwrap()
}

/// Extractor that resolves the hostname of the request.
/// Hostname is resolved through the following, in order:
///
/// Forwarded header
/// X-Forwarded-Host header
/// Host header
/// request target / URI
// Note that user agents can set X-Forwarded-Host and Host headers
// to arbitrary values so make sure to validate them to avoid security issues.
pub fn host_resolver(headers: &HeaderMap) -> anyhow::Result<Hostname> {
    let host = headers
        .get("Host")
        .ok_or(anyhow!("No host header"))?
        .to_str()?;
    Ok(Hostname(host.to_owned()))
}

/// Routing by hostname is a mechanism for isolating API services by giving each API its
/// own hostname; for example, service-a.api.example.com or service-a.example.com.
pub async fn router(
    req: Request<Body>, connection_manager: Arc<ConnectionManager>, ip: SocketAddr,
) -> anyhow::Result<Response<Body>> {
    connection_manager
        .connection_context
        .try_lock()
        .map_err(|_| anyhow::anyhow!("Unable to obtain mutex lock"))?
        .insert(rusty_ulid::generate_ulid_string(), ip.to_string());

    info!(
        "conns {:?}",
        connection_manager.connection_context.try_lock().unwrap()
    );

    let host = host_resolver(req.headers())?;

    match host.0.as_str() {
        "test.hermes.local" => Ok(route_to_hermes(req).await?),
        _ => todo!(),
    }
}

/// Route to hermes backend
async fn route_to_hermes(req: Request<Body>) -> anyhow::Result<Response<Body>> {
    let method = req.method().to_owned();
    let uri = req.uri().to_owned();

    // wire to triggered event lambda instance
    let (lambda_send, lambda_recv_answer) = channel();

    let headers = req.headers().clone();

    // let _event_body: Event = serde_json::from_slice(&req.collect().await?.to_bytes())?;

    match (method, uri.path()) {
        (Method::GET, "/kv") => {
            let event_method = headers
                .get("Method")
                .ok_or(anyhow!("No event method for target app"))?
                .to_str()?;

            let event_params = headers
                .get("Params")
                .ok_or(anyhow!("No event parameters for target app"))?
                .to_str()?;

            match event_method {
                "kv-get" => {
                    let on_kv_get_event = KVGet {
                        sender: lambda_send,
                        event: event_params.to_owned(),
                    };

                    crate::event::queue::send(HermesEvent::new(
                        on_kv_get_event,
                        TargetApp::All,
                        TargetModule::All,
                    ))?;

                    return Ok(Response::new(lambda_recv_answer.recv()?.into()));
                },
                _ => todo!(),
            }
        },
        _ => todo!(),
    };
}
