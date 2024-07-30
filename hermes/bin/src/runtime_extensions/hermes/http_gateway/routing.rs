use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::{
        mpsc::{channel, Receiver, Sender},
        Arc,
    },
    time::Duration,
};

use anyhow::{anyhow, Ok};
use hyper::{
    self,
    body::{Bytes, HttpBody},
    Body, HeaderMap, Request, Response, StatusCode,
};
use regex::Regex;
use tracing::info;

use super::{
    event::{HTTPEvent, HTTPEventMsg, HeadersKV},
    gateway_task::{ClientIPAddr, Config, ConnectionManager, EventUID, LiveConnection, Processed},
    VFS,
};
use crate::event::{HermesEvent, TargetApp, TargetModule};

/// Everything that hits /api routes to Webasm Component Modules
const WEBASM_ROUTE: &str = "/api";

/// Check path is valid for static files
const VALID_PATH: &str = r"^((/[a-zA-Z0-9-_]+)+|/)$";

/// Attempts to wait for a value on this receiver,
/// returning an error if the corresponding channel has hung up,
/// or if it waits more than timeout of arbitrary 1 second
const EVENT_TIMEOUT: u64 = 1;

/// Application name
#[derive(Debug)]
pub(crate) struct AppName(pub String);

/// hostname (node name)
#[derive(Debug)]
pub(crate) struct Hostname(pub String);

/// HTTP error response generator
pub(crate) fn error_response(err: String) -> anyhow::Result<Response<Body>> {
    Ok(Response::builder()
        .status(StatusCode::INTERNAL_SERVER_ERROR)
        .body(err.into())?)
}

/// HTTP not found response generator
pub(crate) fn not_found() -> anyhow::Result<Response<Body>> {
    Ok(Response::builder()
        .status(StatusCode::NOT_FOUND)
        .body("Not Found".into())?)
}

/// Extractor that resolves the hostname of the request.
/// Hostname is resolved through the Host header
pub(crate) fn host_resolver(headers: &HeaderMap) -> anyhow::Result<(AppName, Hostname)> {
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
pub(crate) async fn router(
    req: Request<Body>, connection_manager: Arc<ConnectionManager>, ip: SocketAddr, config: Config,
) -> anyhow::Result<Response<Body>> {
    let unique_request_id = EventUID(rusty_ulid::generate_ulid_string());

    connection_manager
        .get_connection_manager_context()
        .try_lock()
        .map_err(|_| anyhow::anyhow!("Unable to obtain mutex lock"))?
        .insert(
            unique_request_id.clone(),
            (ClientIPAddr(ip), Processed(false), LiveConnection(true)),
        );

    info!("connection manager {:?}", connection_manager);

    let (app_name, resolved_host) = host_resolver(req.headers())?;

    let response = if config
        .valid_hosts
        .iter()
        .any(|host| host.0 == resolved_host.0.as_str())
    {
        route_to_hermes(req).await?
    } else {
        return Ok(error_response("Hostname not valid".to_owned())?);
    };

    connection_manager
        .get_connection_manager_context()
        .try_lock()
        .map_err(|_| anyhow::anyhow!("Unable to obtain mutex lock"))?
        .insert(
            unique_request_id,
            (ClientIPAddr(ip), Processed(true), LiveConnection(false)),
        );

    info!(
        "connection manager {:?} app {:?}",
        connection_manager, app_name.0
    );

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

    if uri.path() == WEBASM_ROUTE {
        compose_http_event(
            method,
            header_map.into_iter().collect(),
            req.collect().await?.to_bytes(), // body
            path,
            lambda_send,
            &lambda_recv_answer,
        )
    } else if is_valid_path(uri.path())? {
        serve_static_data(uri.path())
    } else {
        Ok(not_found()?)
    }
}

/// Compose http event and send to global queue, await queue response and relay back to
/// waiting receiver channel for HTTP response
fn compose_http_event(
    method: String, headers: HeadersKV, body: Bytes, path: String, sender: Sender<HTTPEventMsg>,
    receiver: &Receiver<HTTPEventMsg>,
) -> anyhow::Result<Response<Body>> {
    let on_http_event = HTTPEvent {
        headers,
        method,
        path,
        body,
        sender,
    };

    let event = HermesEvent::new(on_http_event, TargetApp::All, TargetModule::All);

    crate::event::queue::send(event)?;

    match &receiver.recv_timeout(Duration::from_secs(EVENT_TIMEOUT))? {
        HTTPEventMsg::HttpEventResponse(resp) => {
            Ok(Response::new(serde_json::to_string(&resp)?.into()))
        },
        HTTPEventMsg::HTTPEventReceiver => Ok(error_response("HTTP event msg error".to_owned())?),
    }
}

/// Serves static data with 1:1 mapping
fn serve_static_data(path: &str) -> anyhow::Result<Response<Body>> {
    let vfs = VFS
        .get()
        .ok_or(anyhow::anyhow!("Unable to obtain virtual filesystem"))?;

    let file = vfs.read(path)?;

    Ok(Response::new(file.into()))
}

/// Check if valid path to static files.
fn is_valid_path(path: &str) -> anyhow::Result<bool> {
    let regex = Regex::new(VALID_PATH)?;

    Ok(regex.is_match(path))
}

#[cfg(test)]
mod tests {
    use regex::Regex;

    use super::VALID_PATH;

    #[test]
    fn test_valid_paths_regex() {
        // ^ and $: Match the entire string/line
        // (/[a-zA-Z0-9-_]+)+: One or more directories, starting with slash, separated by
        // slashes; each directory must consist of one or more characters of your charset.
        // (...)+|/: Explicitly allow just a single slash
        let regex = Regex::new(VALID_PATH).unwrap();

        // valid
        let a = "/abc/def";
        let b = "/hello_1/world";
        let c = "/three/directories/abc";
        let d = "/";
        let valid_path = vec![a, b, c, d];

        for valid in valid_path {
            if let Some(captures) = regex.captures(valid) {
                assert_eq!(captures.get(0).unwrap().as_str(), valid);
            }
        }

        // invalid
        let a = "/abc/def/";
        let b = "/abc//def";
        let c = "//";
        let d = "abc/def";
        let e = "/abc/def/file.txt";
        let invalids = vec![a, b, c, d, e];

        for invalid in invalids {
            if let Some(captures) = regex.captures(invalid) {
                assert!(captures.len() == 0);
            }
        }
    }
}
