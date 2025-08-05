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
use http_body_util::{BodyExt, Full};
use hyper::{
    self,
    body::{Body, Bytes, Incoming},
    HeaderMap, Request, Response, StatusCode,
};
use regex::Regex;
use tracing::info;

use super::{
    event::{HTTPEvent, HTTPEventMsg, HeadersKV},
    gateway_task::{ClientIPAddr, Config, ConnectionManager, EventUID, LiveConnection, Processed},
};
use crate::{
    app::ApplicationName,
    event::{HermesEvent, TargetApp, TargetModule},
    reactor,
};

/// Everything that hits /api routes to Webasm Component Modules
const WEBASM_ROUTE: &str = "/api";

/// Check path is valid for static files
const VALID_PATH: &str = r"^((/[a-zA-Z0-9-_]+)+|/)$";

/// Attempts to wait for a value on this receiver,
/// returning an error if the corresponding channel has hung up,
/// or if it waits more than timeout of arbitrary 1 second
const EVENT_TIMEOUT: u64 = 1;

/// hostname (node name)
#[derive(Debug)]
pub(crate) struct Hostname(pub String);

/// HTTP error response generator
pub(crate) fn error_response<B>(err: String) -> anyhow::Result<Response<B>>
where B: Body + From<String> {
    Ok(Response::builder()
        .status(StatusCode::INTERNAL_SERVER_ERROR)
        .body(err.into())?)
}

/// HTTP not found response generator
pub(crate) fn not_found<B>() -> anyhow::Result<Response<B>>
where B: Body + From<&'static str> {
    Ok(Response::builder()
        .status(StatusCode::NOT_FOUND)
        .body("Not Found".into())?)
}

/// Extractor that resolves the hostname of the request.
/// Hostname is resolved through the Host header
pub(crate) fn host_resolver(headers: &HeaderMap) -> anyhow::Result<(ApplicationName, Hostname)> {
    let host = headers
        .get("Host")
        .ok_or(anyhow!("No host header"))?
        .to_str()?;

    // <app.name>.hermes.local
    // host = hermes.local
    let (app, host) = host
        .split_once('.')
        .ok_or(anyhow::anyhow!("Malformed Host header"))?;

    Ok((ApplicationName(app.to_owned()), Hostname(host.to_owned())))
}

/// Routing by hostname is a mechanism for isolating API services by giving each API its
/// own hostname; for example, service-a.api.example.com or service-a.example.com.
pub(crate) async fn router(
    req: Request<Incoming>,
    connection_manager: Arc<ConnectionManager>,
    ip: SocketAddr,
    config: Config,
) -> anyhow::Result<Response<Full<Bytes>>> {
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
        route_to_hermes(req, app_name.clone()).await?
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
        connection_manager, app_name
    );

    Ok(response)
}

/// Route single request to hermes backend
async fn route_to_hermes(
    req: Request<Incoming>,
    app_name: ApplicationName,
) -> anyhow::Result<Response<Full<Bytes>>> {
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

    let (_parts, body) = req.into_parts();
    if uri.path() == WEBASM_ROUTE || uri.path().starts_with(&format!("{WEBASM_ROUTE}/")) {
        compose_http_event(
            method,
            header_map.into_iter().collect(),
            body.collect().await?.to_bytes(),
            path,
            lambda_send,
            &lambda_recv_answer,
        )
    } else if is_valid_path(uri.path()).is_ok() {
        serve_static_data(uri.path(), &app_name)
    } else {
        Ok(not_found()?)
    }
}

/// Compose http event and send to global queue, await queue response and relay back to
/// waiting receiver channel for HTTP response
fn compose_http_event<B>(
    method: String,
    headers: HeadersKV,
    body: Bytes,
    path: String,
    sender: Sender<HTTPEventMsg>,
    receiver: &Receiver<HTTPEventMsg>,
) -> anyhow::Result<Response<B>>
where
    B: Body + From<String>,
{
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
fn serve_static_data<B>(
    path: &str,
    app_name: &ApplicationName,
) -> anyhow::Result<Response<B>>
where
    B: Body + From<Vec<u8>>,
{
    let app = reactor::get_app(app_name)?;
    let file = app.vfs().read(path)?;

    Ok(Response::new(file.into()))
}

/// Check if valid path to static files.
fn is_valid_path(path: &str) -> anyhow::Result<()> {
    let regex = Regex::new(VALID_PATH)?;

    if regex.is_match(path) {
        Ok(())
    } else {
        Err(anyhow::anyhow!("Not a valid path {:?}", path))
    }
}

#[cfg(test)]
mod tests {
    use regex::Regex;

    use super::VALID_PATH;
    use crate::runtime_extensions::hermes::http_gateway::routing::is_valid_path;

    #[test]
    fn test_valid_paths_regex() {
        // ^ and $: Match the entire string/line
        // (/[a-zA-Z0-9-_]+)+: One or more directories, starting with slash, separated by
        // slashes; each directory must consist of one or more characters of your charset.
        // (...)+|/: Explicitly allow just a single slash
        let regex = Regex::new(VALID_PATH).unwrap();

        // valid
        let example_one = "/abc/def";
        let example_two = "/hello_1/world";
        let example_three = "/three/directories/abc";
        let example_four = "/";
        let valid_path = vec![example_one, example_two, example_three, example_four];

        for valid in valid_path {
            if let Some(captures) = regex.captures(valid) {
                assert_eq!(captures.get(0).unwrap().as_str(), valid);
            }

            assert!(is_valid_path(valid).is_ok());
        }

        // invalid
        let example_one = "/abc/def/";
        let example_two = "/abc//def";
        let example_three = "//";
        let example_four = "abc/def";
        let example_five = "/abc/def/file.txt";
        let invalids = vec![
            example_one,
            example_two,
            example_three,
            example_four,
            example_five,
        ];

        for invalid in invalids {
            if let Some(captures) = regex.captures(invalid) {
                assert!(captures.len() == 0);
            }

            assert!(is_valid_path(invalid).is_err());
        }
    }
}
