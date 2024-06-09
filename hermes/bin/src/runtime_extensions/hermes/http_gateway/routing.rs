use std::{
    net::SocketAddr,
    sync::{
        mpsc::{channel, Receiver, Sender},
        Arc,
    },
};

use anyhow::{anyhow, Ok};
use hyper::{
    self,
    body::{Bytes, HttpBody},
    Body, HeaderMap, Request, Response, StatusCode,
};
use serde::{Deserialize, Serialize};
use tracing::info;

use super::{
    event::{HTTPEvent, HTTPEventMsg},
    gateway_task::{ClientIPAddr, ConnectionManager, EventUID, LiveConnection, Processed},
};
use crate::event::{HermesEvent, TargetApp, TargetModule};

#[derive(Serialize, Deserialize, Debug)]
struct Headers<K, V> {
    contents: Vec<(K, V)>,
}

#[derive(Debug)]
/// hostname (nodename)
pub struct Hostname(pub String);

/// HTTP error response generator
pub fn _error_response(err: String) -> Response<Body> {
    Response::builder()
        .status(StatusCode::INTERNAL_SERVER_ERROR)
        .body(err.into())
        .unwrap()
}

/// HTTP not found response generator
pub fn _not_found() -> Response<Body> {
    Response::builder()
        .status(StatusCode::NOT_FOUND)
        .body("Not Found".into())
        .unwrap()
}

/// Extractor that resolves the hostname of the request.
/// Hostname is resolved through the Host header
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
    let unique_request_id = EventUID(rusty_ulid::generate_ulid_string());

    connection_manager
        .connection_context
        .try_lock()
        .map_err(|_| anyhow::anyhow!("Unable to obtain mutex lock"))?
        .insert(
            unique_request_id.clone(),
            (ClientIPAddr(ip), Processed(false), LiveConnection(true)),
        );

    info!("connection manager {:?}", connection_manager);

    let host = host_resolver(req.headers())?;

    let response = match host.0.as_str() {
        "app.hermes.local" => route(req).await?,
        _ => todo!(),
    };

    connection_manager
        .connection_context
        .try_lock()
        .map_err(|_| anyhow::anyhow!("Unable to obtain mutex lock"))?
        .insert(
            unique_request_id,
            (ClientIPAddr(ip), Processed(true), LiveConnection(false)),
        );

    info!("connection manager {:?}\n", connection_manager);

    Ok(response)
}

/// Route single request to hermes backend
async fn route(req: Request<Body>) -> anyhow::Result<Response<Body>> {
    let (lambda_send, lambda_recv_answer): (Sender<HTTPEventMsg>, Receiver<HTTPEventMsg>) =
        channel();

    let uri = req.uri().to_owned();
    let method = req.method().to_owned().to_string();
    let path = req.uri().path().to_string();

    let mut header_kv = Headers {
        contents: Vec::new(),
    };

    for header in req.headers() {
        header_kv
            .contents
            .push((header.0.as_str().to_owned(), header.1.to_str()?.to_owned()))
    }
    let header_bytes = bincode::serialize(&header_kv)?;

    let body = &req.collect().await?.to_bytes();

    match uri.path() {
        "/api" => {
            compose_http_event(
                method,
                header_bytes,
                body.clone(),
                path,
                lambda_send,
                lambda_recv_answer,
            )
        },
        _ => todo!(),
    }
}

/// Compose http event and send to global queue, await queue response and relay back to
/// waiting receiver channel for HTTP response
fn compose_http_event(
    method: String, headers: Vec<u8>, body: Bytes, path: String, sender: Sender<HTTPEventMsg>,
    receiver: Receiver<HTTPEventMsg>,
) -> anyhow::Result<Response<Body>> {
    let on_http_event = HTTPEvent {
        headers,
        method,
        path,
        body,
        sender,
    };
    crate::event::queue::send(HermesEvent::new(
        on_http_event,
        TargetApp::All,
        TargetModule::All,
    ))?;

    return Ok(Response::new(
        serde_json::to_string(&receiver.recv()?)?.into(),
    ));
}
