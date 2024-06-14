use std::{
    collections::HashMap,
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
use tracing::info;

use super::{
    event::{HTTPEvent, HTTPEventMsg, HeadersKV},
    gateway_task::{ClientIPAddr, Config, ConnectionManager, EventUID, LiveConnection, Processed},
};
use crate::event::{HermesEvent, TargetApp, TargetModule};

/// Application name
pub struct AppName(pub String);

#[derive(Debug)]
/// hostname (nodename)
pub struct Hostname(pub String);

/// HTTP error response generator
pub fn error_response(err: String) -> Response<Body> {
    Response::builder()
        .status(StatusCode::INTERNAL_SERVER_ERROR)
        .body(err.into())
        .expect("Infallible")
}

/// HTTP not found response generator
pub(crate) fn not_found() -> Response<Body> {
    Response::builder()
        .status(StatusCode::NOT_FOUND)
        .body("Not Found".into())
        .expect("Infallible")
}

/// Extractor that resolves the hostname of the request.
/// Hostname is resolved through the Host header
pub fn host_resolver(headers: &HeaderMap) -> anyhow::Result<(AppName, Hostname)> {
    let host = headers
        .get("Host")
        .ok_or(anyhow!("No host header"))?
        .to_str()?;

    // <app.name>.hermes.local
    // host = hermes.local
    let (app, host) = host
        .split_once('.')
        .ok_or(anyhow::anyhow!("Malformed Host header"))?;

    Ok((AppName(app.to_owned()), Hostname(host.to_owned())))
}

/// Routing by hostname is a mechanism for isolating API services by giving each API its
/// own hostname; for example, service-a.api.example.com or service-a.example.com.
pub async fn router(
    req: Request<Body>, connection_manager: Arc<ConnectionManager>, ip: SocketAddr, config: Config,
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

    let (_app_name, resolved_host) = host_resolver(req.headers())?;

    let response = if config
        .valid_hosts
        .iter()
        .any(|host| host.0 == resolved_host.0.as_str())
    {
        route_to_hermes(req).await?
    } else {
        return Ok(error_response("hostname not valid".to_owned()));
    };

    connection_manager
        .connection_context
        .try_lock()
        .map_err(|_| anyhow::anyhow!("Unable to obtain mutex lock"))?
        .insert(
            unique_request_id,
            (ClientIPAddr(ip), Processed(true), LiveConnection(false)),
        );

    info!("connection manager {:?}", connection_manager);

    Ok(response)
}

/// Route single request to hermes backend
async fn route_to_hermes(req: Request<Body>) -> anyhow::Result<Response<Body>> {
    let (lambda_send, lambda_recv_answer): (Sender<HTTPEventMsg>, Receiver<HTTPEventMsg>) =
        channel();

    let uri = req.uri().to_owned();
    let method = req.method().to_owned().to_string();
    let path = req.uri().path().to_string();

    let mut header_map: HashMap<String, Vec<String>> = HashMap::new();

    for (header_name, header_val) in req.headers() {
        header_map
            .entry(header_name.to_string())
            .or_default()
            .push(header_val.to_str()?.to_string());
    }

    match uri.path() {
        "/api" => {
            compose_http_event(
                method,
                header_map.into_iter().collect(),
                req.collect().await?.to_bytes(), // body
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
    method: String, headers: HeadersKV, body: Bytes, path: String, sender: Sender<HTTPEventMsg>,
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

    match &receiver.recv()? {
        HTTPEventMsg::HttpEventResponseSome(resp) => {
            Ok(Response::new(serde_json::to_string(&resp)?.into()))
        },
        HTTPEventMsg::HttpEventResponseNone() => Ok(not_found()),

        _ => Ok(error_response("HTTP event msg error".to_owned())),
    }
}
