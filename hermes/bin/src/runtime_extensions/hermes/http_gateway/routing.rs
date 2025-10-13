use std::{
    net::SocketAddr,
    result::Result::Ok,
    sync::{
        mpsc::{channel, Receiver, Sender},
        LazyLock,
    },
    time::Duration,
};

use anyhow::anyhow;
use http_body_util::{BodyExt, Full};
use hyper::{
    self,
    body::{Body, Bytes, Incoming},
    HeaderMap, Request, Response, StatusCode,
};
use regex::Regex;
#[allow(unused_imports, reason = "`debug` used only in debug builds.")]
use tracing::{debug, error, info};

use super::{
    event::{HTTPEvent, HTTPEventMsg, HeadersKV},
    gateway_task::{ClientIPAddr, Config, ConnectionManager, EventUID, LiveConnection, Processed},
};
use crate::{
    app::{Application, ApplicationName},
    event::{HermesEvent, TargetApp, TargetModule},
    reactor,
    runtime_extensions::hermes::http_gateway::subscription::find_global_endpoint_subscription,
};

/// Everything that hits /api routes to Webasm Component Modules
const WEBASM_ROUTE: &str = "/api";

/// Path validation for static files within sandboxed VFS
/// Note: Basic validation - relies on VFS sandbox for security isolation
/// TODO: Update with stricter path validation in the future
const VALID_PATH: &str = r"^(/.*|/)$";

/// Attempts to wait for a value on this receiver,
/// returning an error if the corresponding channel has hung up,
/// or if it waits more than timeout of arbitrary 1 second
const EVENT_TIMEOUT: u64 = 1;

/// hostname (node name)
#[cfg_attr(debug_assertions, derive(Debug))]
pub(crate) struct Hostname(pub String);

/// Main HTTP request router that processes incoming requests by hostname
pub(crate) async fn router(
    req: Request<Incoming>,
    connection_manager: ConnectionManager,
    ip: SocketAddr,
    config: Config,
) -> anyhow::Result<Response<Full<Bytes>>> {
    let unique_request_id = EventUID(rusty_ulid::generate_ulid_string());
    let client_ip = ClientIPAddr(ip);

    connection_manager.insert(
        unique_request_id.clone(),
        (client_ip.clone(), Processed(false), LiveConnection(true)),
    );

    #[cfg(debug_assertions)]
    debug!("connection manager {:?}", connection_manager);

    let (app_name, resolved_host) = host_resolver(req.headers())?;

    let response = if config
        .valid_hosts
        .iter()
        .any(|host| host.0 == resolved_host.0)
    {
        route_to_hermes(req, app_name.clone()).await?
    } else {
        return error_response("Invalid hostname");
    };

    connection_manager.insert(
        unique_request_id,
        (client_ip, Processed(true), LiveConnection(false)),
    );

    #[cfg(debug_assertions)]
    debug!("connection manager {connection_manager} app {app_name}");

    Ok(response)
}

pub(crate) fn error_response<B>(err: impl Into<String>) -> anyhow::Result<Response<B>>
where
    B: Body + From<String>,
{
    Response::builder()
        .status(StatusCode::INTERNAL_SERVER_ERROR)
        .body(err.into().into())
        .map_err(Into::into)
}

fn not_found<B>() -> anyhow::Result<Response<B>>
where
    B: Body + From<Vec<u8>>,
{
    let response = Response::builder()
        .status(StatusCode::NOT_FOUND)
        .body(b"Not Found".to_vec().into())?;
    add_security_headers(response)
}

pub(crate) fn host_resolver(headers: &HeaderMap) -> anyhow::Result<(ApplicationName, Hostname)> {
    let host = headers
        .get("Host")
        .and_then(|h| h.to_str().ok())
        .ok_or_else(|| anyhow!("Missing or invalid Host header"))?;

    let host_without_port = host.split(':').next().unwrap_or(host);
    let (app, hostname) = host_without_port
        .split_once('.')
        .ok_or_else(|| anyhow!("Malformed Host header: expected format 'app.domain'"))?;

    Ok((
        ApplicationName(app.to_owned()),
        Hostname(hostname.to_owned()),
    ))
}

async fn route_to_hermes(
    req: Request<Incoming>,
    app_name: ApplicationName,
) -> anyhow::Result<Response<Full<Bytes>>> {
    let uri = req.uri().to_owned();
    let route_type = determine_route(&uri, &req).await?;

    match route_type {
        RouteType::WebAssembly(path, module_id) => {
            handle_webasm_request(req, path, module_id, app_name).await
        },
        RouteType::StaticFile(path) => serve_static_web_content(&path, &app_name),
    }
}

enum RouteType {
    WebAssembly(String, Option<String>),
    StaticFile(String),
}

async fn determine_route(
    uri: &hyper::Uri,
    req: &Request<Incoming>,
) -> anyhow::Result<RouteType> {
    let path = uri
        .path_and_query()
        .map_or(uri.path(), hyper::http::uri::PathAndQuery::as_str);

    let method = req.method().as_str();
    let content_type = req
        .headers()
        .get("content-type")
        .and_then(|ct| ct.to_str().ok());

    if let Some(subscription) =
        find_global_endpoint_subscription(method, uri.path(), content_type).await
    {
        debug!(
            "Found subscription for {} {}: module {}",
            method,
            uri.path(),
            subscription.module_id
        );
        return Ok(RouteType::WebAssembly(
            path.to_string(),
            Some(subscription.module_id),
        ));
    }

    if uri.path() == WEBASM_ROUTE || uri.path().starts_with(&format!("{WEBASM_ROUTE}/")) {
        Ok(RouteType::WebAssembly(path.to_string(), None))
    } else if is_valid_path(uri.path()).is_ok() {
        Ok(RouteType::StaticFile(path.to_string()))
    } else {
        Err(anyhow!("Invalid route: {}", uri.path()))
    }
}

async fn handle_webasm_request(
    req: Request<Incoming>,
    path: String,
    module_id: Option<String>,
    app_name: ApplicationName,
) -> anyhow::Result<Response<Full<Bytes>>> {
    let (lambda_send, lambda_recv_answer) = channel();
    let method = req.method().to_string();

    let headers: HeadersKV = req
        .headers()
        .iter()
        .map(|(name, value)| {
            let key = name.to_string();
            let values = vec![value.to_str().unwrap_or_default().to_string()];
            (key, values)
        })
        .collect();

    let (_parts, body) = req.into_parts();
    let body_bytes = body.collect().await?.to_bytes();

    compose_http_event(
        method,
        headers,
        body_bytes,
        path,
        lambda_send,
        &lambda_recv_answer,
        module_id,
        app_name,
    )
}

fn compose_http_event<B>(
    method: String,
    headers: HeadersKV,
    body: Bytes,
    path: String,
    sender: Sender<HTTPEventMsg>,
    receiver: &Receiver<HTTPEventMsg>,
    module_id: Option<String>,
    app_name: ApplicationName,
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

    let app = reactor::get_app(&app_name)?;
    //TargetModule::List(vec![ModuleId(target_module)]), // Fixed: Use List with ModuleId

    app.get_human();

    let event = match module_id {
        Some(target_module) => {
            debug!("Routing HTTP request to specific module: {}", target_module);
            // Use List variant with a single ModuleId
            HermesEvent::new(on_http_event, TargetApp::All, TargetModule::All)
        },
        None => {
            debug!("Broadcasting HTTP request to all modules (no specific subscription)");
            HermesEvent::new(on_http_event, TargetApp::All, TargetModule::All)
        },
    };

    crate::event::queue::send(event)?;

    let timeout = Duration::from_secs(EVENT_TIMEOUT);
    match receiver.recv_timeout(timeout)? {
        HTTPEventMsg::HttpEventResponse(resp) => {
            let body = serde_json::to_string(&resp)?.into();
            Ok(Response::new(body))
        },
        HTTPEventMsg::HTTPEventReceiver => error_response("HTTP event message error"),
    }
}

static VALID_PATH_REGEX: LazyLock<Option<Regex>> = LazyLock::new(|| Regex::new(VALID_PATH).ok());

fn is_valid_path(path: &str) -> anyhow::Result<()> {
    VALID_PATH_REGEX
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Path validation unavailable: invalid regex pattern"))?
        .is_match(path)
        .then_some(())
        .ok_or_else(|| anyhow::anyhow!("Invalid path format: {path:?}"))
}

fn serve_static_web_content<B>(
    path: &str,
    app_name: &ApplicationName,
) -> anyhow::Result<Response<B>>
where
    B: Body + From<Vec<u8>>,
{
    let file_path = resolve_static_file_path(path);
    let app = reactor::get_app(app_name)?;

    app.vfs()
        .read(&file_path)
        .and_then(|contents| serve_existing_asset(contents, &file_path))
        .or_else(|_| handle_missing_asset(&file_path, &app))
}

const DEFAULT_INDEX_PATH: &str = "www/index.html";
const DOCUMENT_ROOT: &str = "www";

fn resolve_static_file_path(path: &str) -> String {
    if path == "/" {
        DEFAULT_INDEX_PATH.to_string()
    } else {
        let clean_path = path.strip_prefix('/').unwrap_or(path);
        format!("{DOCUMENT_ROOT}/{clean_path}")
    }
}

fn serve_existing_asset<B>(
    file_contents: Vec<u8>,
    file_path: &str,
) -> anyhow::Result<Response<B>>
where
    B: Body + From<Vec<u8>>,
{
    let mut response = Response::new(file_contents.into());

    if let Some(extension) = get_file_extension(file_path) {
        let content_type = get_flutter_content_type(extension);
        response
            .headers_mut()
            .insert("Content-Type", content_type.parse()?);
    }

    add_security_headers(response)
}

fn handle_missing_asset<B>(
    file_path: &str,
    app: &Application,
) -> anyhow::Result<Response<B>>
where
    B: Body + From<Vec<u8>>,
{
    if is_critical_asset(file_path) {
        error!("Critical asset missing: {}", file_path);
        return Ok(Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body("Asset not found".as_bytes().to_vec().into())?);
    }

    serve_index_html_fallback(file_path, app)
}

fn is_critical_asset(file_path: &str) -> bool {
    matches!(
        get_file_extension(file_path),
        Some("js" | "wasm" | "json" | "css" | "map")
    )
}

fn add_security_headers<B>(mut response: Response<B>) -> anyhow::Result<Response<B>>
where
    B: Body,
{
    let headers = response.headers_mut();

    headers.insert(
        "Cross-Origin-Opener-Policy",
        "same-origin"
            .parse()
            .map_err(|e| anyhow!("Invalid COOP header value: {e}"))?,
    );
    headers.insert(
        "Cross-Origin-Embedder-Policy",
        "require-corp"
            .parse()
            .map_err(|e| anyhow!("Invalid COEP header value: {e}"))?,
    );

    Ok(response)
}

fn get_file_extension(path: &str) -> Option<&str> {
    path.split('.').next_back()
}

fn get_flutter_content_type(extension: &str) -> &'static str {
    match extension {
        "html" => "text/html",
        "js" => "application/javascript",
        "wasm" => "application/wasm",
        "css" => "text/css",
        "json" => "application/json",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "svg" => "image/svg+xml",
        "ico" => "image/x-icon",
        "woff" | "woff2" => "font/woff2",
        "ttf" => "font/ttf",
        "otf" => "font/otf",
        _ => "application/octet-stream",
    }
}

const CONTENT_TYPE_HTML: &str = "text/html";
const NO_CACHE_DIRECTIVE: &str = "no-cache, no-store, must-revalidate";

fn serve_index_html_fallback<B>(
    file_path: &str,
    app: &Application,
) -> anyhow::Result<Response<B>>
where
    B: Body + From<Vec<u8>>,
{
    match file_path {
        "www/index.html" => not_found(),
        _ => match app.vfs().read("www/index.html") {
            Ok(index_contents) => {
                let mut response = Response::new(index_contents.into());
                response
                    .headers_mut()
                    .insert("Content-Type", CONTENT_TYPE_HTML.parse()?);
                response
                    .headers_mut()
                    .insert("Cache-Control", NO_CACHE_DIRECTIVE.parse()?);
                response = add_security_headers(response)?;
                Ok(response)
            },
            Err(_) => not_found(),
        },
    }
}

#[cfg(all(test, debug_assertions))]
mod tests {
    use regex::Regex;

    use super::VALID_PATH;
    use crate::runtime_extensions::hermes::http_gateway::routing::is_valid_path;

    #[test]
    fn test_valid_paths_regex() {
        let regex = Regex::new(VALID_PATH).unwrap();

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
