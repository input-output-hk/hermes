use std::result::Result::Ok;
use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::mpsc::{channel, Receiver, Sender},
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
    app::ApplicationName,
    event::{HermesEvent, TargetApp, TargetModule},
    reactor,
};

/// Everything that hits /api routes to Webasm Component Modules
const WEBASM_ROUTE: &str = "/api";

/// Check path is valid for static files
const VALID_PATH: &str = r"^(/.*|/)$";

/// Attempts to wait for a value on this receiver,
/// returning an error if the corresponding channel has hung up,
/// or if it waits more than timeout of arbitrary 1 second
const EVENT_TIMEOUT: u64 = 1;

/// hostname (node name)
#[cfg_attr(debug_assertions, derive(Debug))]
pub(crate) struct Hostname(pub String);

/// HTTP error response generator
pub(crate) fn error_response<B>(err: String) -> anyhow::Result<Response<B>>
where
    B: Body + From<String>,
{
    Ok(Response::builder()
        .status(StatusCode::INTERNAL_SERVER_ERROR)
        .body(err.into())?)
}

/// HTTP not found response generator
pub(crate) fn not_found<B>() -> anyhow::Result<Response<B>>
where
    B: Body + From<&'static str>,
{
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

    // Strip port if present (e.g., "app.hermes.local:5000" -> "app.hermes.local")
    let host = host.split(':').next().unwrap_or(host);

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
    connection_manager: ConnectionManager,
    ip: SocketAddr,
    config: Config,
) -> anyhow::Result<Response<Full<Bytes>>> {
    let unique_request_id = EventUID(rusty_ulid::generate_ulid_string());

    connection_manager.insert(
        unique_request_id.clone(),
        (ClientIPAddr(ip), Processed(false), LiveConnection(true)),
    );

    #[cfg(debug_assertions)]
    debug!("connection manager {:?}", connection_manager);

    let (app_name, resolved_host) = host_resolver(req.headers())?;

    let response = if config
        .valid_hosts
        .iter()
        .any(|host| host.0 == resolved_host.0.as_str())
    {
        route_to_hermes(req, app_name.clone()).await?
    } else {
        return error_response("Hostname not valid".to_owned());
    };

    connection_manager.insert(
        unique_request_id,
        (ClientIPAddr(ip), Processed(true), LiveConnection(false)),
    );

    #[cfg(debug_assertions)]
    debug!("connection manager {connection_manager} app {app_name}");

    Ok(response)
}

/// Routes HTTP requests to WASM modules or static file handlers
///
/// Converts incoming HTTP requests into structured events for WASM processing,
/// preserving full URLs including query parameters for accurate forwarding.
///
/// ## Routing
/// - `/api/*` → WASM modules via event queue
/// - Valid paths → Static file system
/// - Invalid paths → HTTP 404
///
/// ## Key Features
/// - Preserves query parameters (e.g., `?asat=SLOT:95022059`)
/// - Multi-value header support
/// - Async request/response via MPSC channels
async fn route_to_hermes(
    req: Request<Incoming>,
    app_name: ApplicationName,
) -> anyhow::Result<Response<Full<Bytes>>> {
    // Create synchronous MPSC channel for receiving WASM module responses
    // Used in request-response pattern: HTTP request → global event queue → WASM modules →
    // response channel TODO: Replace with oneshot channel since we only expect one
    // response per HTTP request
    let (lambda_send, lambda_recv_answer): (Sender<HTTPEventMsg>, Receiver<HTTPEventMsg>) =
        channel();

    let uri = req.uri().to_owned();
    let method = req.method().to_owned().to_string();

    // Include query parameters in path (crucial for redirects)
    let path = uri
        .path_and_query()
        .map_or(uri.path(), hyper::http::uri::PathAndQuery::as_str)
        .to_string();

    // Convert headers to multi-value format
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
        serve_flutter_assets(&path, &app_name)
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

/// Check if valid path to static files.
fn is_valid_path(path: &str) -> anyhow::Result<()> {
    let regex = Regex::new(VALID_PATH)?;

    if regex.is_match(path) {
        Ok(())
    } else {
        Err(anyhow::anyhow!("Not a valid path {:?}", path))
    }
}

/// Serves Flutter web build assets with proper MIME types
fn serve_flutter_assets<B>(
    path: &str,
    app_name: &ApplicationName,
) -> anyhow::Result<Response<B>>
where
    B: Body + From<Vec<u8>>,
{
    let file_path = if path == "/" {
        "www/index.html".to_string()
    } else {
        let clean_path = path.strip_prefix('/').unwrap_or(path);
        format!("www/{clean_path}")
    };

    info!("serving static file @:{}", file_path);

    // ADD THIS LOGGING
    if file_path.contains("flutter_bootstrap.js") {
        info!("🔍 DEBUGGING flutter_bootstrap.js request:");
        info!("   Original path: {}", path);
        info!("   Computed file_path: {}", file_path);
    }

    let app = reactor::get_app(app_name)?;

    if let Ok(file_contents) = app.vfs().read(&file_path) {
        let mut response = Response::new(file_contents.into());

        if let Some(extension) = get_file_extension(&file_path) {
            let content_type = get_flutter_content_type(extension);
            info!(
                "File: {} -> Extension: {} -> Content-Type: {}",
                file_path, extension, content_type
            );
            response
                .headers_mut()
                .insert("Content-Type", content_type.parse()?);
        }

        add_flutter_cache_headers(&mut response, &file_path)?;
        response = add_security_headers(response)?;
        Ok(response)
    } else {
        // NEW: Don't fall back to index.html for JavaScript/WASM files
        if let Some(_extension @ ("js" | "wasm" | "json" | "css" | "map")) = get_file_extension(&file_path) {
            error!("Critical asset missing: {}", file_path);
            return Ok(Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body("Asset not found".as_bytes().to_vec().into())?);
        }

        // Only fall back to index.html for navigation routes (no extension or HTML files)
        match file_path.as_str() {
            "www/index.html" => serve_flutter_not_found(),
            _ => match app.vfs().read("www/index.html") {
                Ok(index_contents) => {
                    let mut response = Response::new(index_contents.into());
                    response
                        .headers_mut()
                        .insert("Content-Type", "text/html".parse()?);
                    response.headers_mut().insert(
                        "Cache-Control",
                        "no-cache, no-store, must-revalidate".parse()?,
                    );
                    response = add_security_headers(response)?;
                    Ok(response)
                },
                Err(_) => serve_flutter_not_found(),
            },
        }
    }
}

/// Add security headers to response
fn add_security_headers<B>(mut response: Response<B>) -> anyhow::Result<Response<B>>
where
    B: Body,
{
    let headers = response.headers_mut();

    // Add COOP and COEP headers for enabling SharedArrayBuffer and other features
    headers.insert(
        "Cross-Origin-Opener-Policy",
        "same-origin"
            .parse()
            .map_err(|e| anyhow!("Invalid COOP header value: {}", e))?,
    );
    headers.insert(
        "Cross-Origin-Embedder-Policy",
        "require-corp"
            .parse()
            .map_err(|e| anyhow!("Invalid COEP header value: {}", e))?,
    );

    Ok(response)
}

/// Extract file extension from path for MIME type detection
fn get_file_extension(path: &str) -> Option<&str> {
    path.split('.').next_back()
}

/// Returns the appropriate MIME type for Flutter web assets based on file extension
///
/// Maps common web asset file extensions to their corresponding MIME types
/// for proper browser handling. Defaults to `application/octet-stream` for
/// unknown extensions.
fn get_flutter_content_type(extension: &str) -> &'static str {
    match extension {
        "html" => "text/html",
        "js" => "application/javascript",
        "dart" => "application/dart",
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

/// Adds appropriate cache headers to Flutter asset responses
///
/// Sets long-term caching for static assets (1 year) but no caching for
/// index.html and service worker files to ensure proper application updates.
fn add_flutter_cache_headers<B>(
    response: &mut Response<B>,
    file_path: &str,
) -> anyhow::Result<()> {
    if !file_path.ends_with("index.html") && !file_path.ends_with("flutter_service_worker.js") {
        response.headers_mut().insert(
            "Cache-Control",
            "public, max-age=31536000"
                .parse()
                .map_err(|e| anyhow!("Invalid cache control header: {}", e))?,
        );
    } else {
        response.headers_mut().insert(
            "Cache-Control",
            "no-cache, no-store, must-revalidate"
                .parse()
                .map_err(|e| anyhow!("Invalid cache control header: {}", e))?,
        );
    }
    Ok(())
}

/// HTTP not found response generator for Flutter assets
fn serve_flutter_not_found<B>() -> anyhow::Result<Response<B>>
where
    B: Body + From<Vec<u8>>,
{
    let response = Response::builder()
        .status(StatusCode::NOT_FOUND)
        .body("Not Found".as_bytes().to_vec().into())?;
    add_security_headers(response)
}

#[cfg(all(test, debug_assertions))]
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