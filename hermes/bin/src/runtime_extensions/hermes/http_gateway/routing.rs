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

use crate::event::{HermesEvent, TargetApp, TargetModule};

use super::{event::HTTPEvent, gateway_task::ConnectionManager};

#[derive(Serialize, Deserialize, Debug)]
struct Headers<K, V> {
    contents: Vec<(K, V)>,
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
    connection_manager
        .connection_context
        .try_lock()
        .map_err(|_| anyhow::anyhow!("Unable to obtain mutex lock"))?
        .insert(rusty_ulid::generate_ulid_string(), ip.to_string());

    let host = host_resolver(req.headers())?;

    match host.0.as_str() {
        "app.hermes.local" => Ok(route(req).await?),
        _ => todo!(),
    }
}

/// Route single request to hermes backend
async fn route(req: Request<Body>) -> anyhow::Result<Response<Body>> {
    let uri = req.uri().to_owned();

    // wire to triggered event lambda instance
    let (lambda_send, lambda_recv_answer) = channel();

    let method = req.method().to_owned().to_string();

    let mut header_kv = Headers {
        contents: Vec::new(),
    };

    for header in req.headers() {
        header_kv.contents.push((
            header.0.as_str().to_owned(),
            header.1.to_str().unwrap().to_owned(),
        ))
    }

    let header_bytes = bincode::serialize(&header_kv)?;

    let body = &req.collect().await?.to_bytes();

    match uri.path() {
        "/api" => compose_http_event(
            method,
            header_bytes,
            body.clone(),
            lambda_send,
            lambda_recv_answer,
        ),
        _ => todo!(),
    }
}

fn compose_http_event(
    method: String, headers: Vec<u8>, body: Bytes, sender: Sender<String>,
    receiver: Receiver<String>,
) -> anyhow::Result<Response<Body>> {
    let on_http_event = HTTPEvent {
        method,
        headers,
        body,
        sender,
    };
    crate::event::queue::send(HermesEvent::new(
        on_http_event,
        TargetApp::All,
        TargetModule::All,
    ))?;

    return Ok(Response::new(receiver.recv()?.into()));
}
