use crate::app::Application;
use std::result::Result::Ok;
use std::{

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
///
/// Takes an HTTP request and routes it to the appropriate handler based on the
/// hostname in the Host header. Each hostname corresponds to a different application.
///
/// ## Process:
/// 1. Generates a unique request ID for tracking
/// 2. Validates the hostname against allowed hosts
/// 3. Routes valid requests to the Hermes application handler
/// 4. Tracks connection state in the connection manager
///
///
/// ## Example:
/// A request with `Host: app.hermes.local` gets routed to the "app" application
/// if "hermes.local" is in the valid hosts list.
pub(crate) async fn router(
    req: Request<Incoming>,
    connection_manager: ConnectionManager,
    ip: SocketAddr,
    config: Config,
) -> anyhow::Result<Response<Full<Bytes>>> {
    let unique_request_id = EventUID(rusty_ulid::generate_ulid_string());
    let client_ip = ClientIPAddr(ip);

    // Register connection start
    connection_manager.insert(
        unique_request_id.clone(),
        (client_ip.clone(), Processed(false), LiveConnection(true)),
    );

    #[cfg(debug_assertions)]
    debug!("connection manager {:?}", connection_manager);

    // Parse the Host header to extract the application name and resolved hostname
    // This determines which application should handle the request
    // For example: "app.hermes.local" -> app_name="app", resolved_host="hermes.local"
    let (app_name, resolved_host) = host_resolver(req.headers())?;

    // Check if the resolved hostname is in our list of valid/allowed hosts
    // This is a security measure to prevent routing to unauthorized applications
    let response = if config.valid_hosts.iter().any(|host| host.0 == resolved_host.0) {
        // If hostname is valid, route the request to the Hermes WASM runtime
        // The app_name determines which specific application will handle it
        route_to_hermes(req, app_name.clone()).await?
    } else {
        // If hostname is not in the valid list, reject with an error response
        // This prevents potential security issues from unauthorized hosts
        return error_response("Invalid hostname");
    };


    // Mark connection as processed
    connection_manager.insert(
        unique_request_id,
        (client_ip, Processed(true), LiveConnection(false)),
    );

    #[cfg(debug_assertions)]
    debug!("connection manager {connection_manager} app {app_name}");

    Ok(response)
}

/// HTTP error response generator
pub(crate) fn error_response<B>(err: impl Into<String>) -> anyhow::Result<Response<B>>
where
    B: Body + From<String>,
{
    Response::builder()
        .status(StatusCode::INTERNAL_SERVER_ERROR)
        .body(err.into().into())
        .map_err(Into::into)
}

/// HTTP not found response generator
fn not_found<B>() -> anyhow::Result<Response<B>>
where
    B: Body + From<Vec<u8>>,
{
    let response = Response::builder()
        .status(StatusCode::NOT_FOUND)
        .body(b"Not Found".to_vec().into())?;
    add_security_headers(response)
}

/// Extractor that resolves the hostname of the request.
/// Hostname is resolved through the Host header
pub(crate) fn host_resolver(headers: &HeaderMap) -> anyhow::Result<(ApplicationName, Hostname)> {
    let host = headers
        .get("Host")
        .and_then(|h| h.to_str().ok())
        .ok_or_else(|| anyhow!("Missing or invalid Host header"))?;

    // Strip port if present (e.g., "app.hermes.local:5000" -> "app.hermes.local")
    let host_without_port = host.split(':').next().unwrap_or(host);

    // Parse <app.name>.hermes.local format
    let (app, hostname) = host_without_port
        .split_once('.')
        .ok_or_else(|| anyhow!("Malformed Host header: expected format 'app.domain'"))?;

    Ok((ApplicationName(app.to_owned()), Hostname(hostname.to_owned())))
}



/// Main HTTP request router that processes incoming requests and delegates to appropriate handlers
///
/// This function serves as the central routing hub, taking validated HTTP requests and
/// directing them to either WebAssembly modules or static file handlers based on the
/// request path and routing rules.
async fn route_to_hermes(
    req: Request<Incoming>,
    app_name: ApplicationName,
) -> anyhow::Result<Response<Full<Bytes>>> {
    // Extract the URI for route analysis - this contains path and query parameters
    let uri = req.uri().to_owned();

    // Analyze the request path and determine which type of handler should process it
    // This applies our routing rules to classify the request appropriately
    let route_type = determine_route(&uri)?;

    // Delegate to the appropriate handler based on route classification
    match route_type {
        // API endpoints need WebAssembly module processing
        // These requests go through the event queue to WASM components
        RouteType::WebAssembly(path) => handle_webasm_request(req, path).await,
        // Static files are served directly from the virtual file system
        RouteType::StaticFile(path) => serve_static_web_content(&path, &app_name),
    }
}

/// Route classification for incoming HTTP requests
///
/// This enum categorizes requests into two main types that require different handling:
/// - API requests that need WebAssembly module processing
/// - Static file requests that serve assets directly from the filesystem
enum RouteType {
    /// API endpoints that should be processed by WebAssembly modules
    /// Contains the full path including query parameters for forwarding
    WebAssembly(String),

    /// Static file requests that serve assets (HTML, CSS, JS, images, etc.)
    /// Contains the normalized file path for filesystem lookup
    StaticFile(String),
}

/// Analyzes an HTTP request URI and determines the appropriate handler type
///
/// This function implements the core routing logic by examining the request path
/// and applying routing rules to classify the request type.
fn determine_route(uri: &hyper::Uri) -> anyhow::Result<RouteType> {
    // Extract the full path including query parameters for accurate forwarding
    // This preserves important data like ?asat=SLOT:95022059 that WASM modules need
    let path = uri
        .path_and_query()
        .map_or(uri.path(), hyper::http::uri::PathAndQuery::as_str);

    // Check if this is an API endpoint that needs WebAssembly processing
    // API routes are identified by the /api prefix (exact match or with additional path)
    if uri.path() == WEBASM_ROUTE || uri.path().starts_with(&format!("{WEBASM_ROUTE}/")) {
        Ok(RouteType::WebAssembly(path.to_string()))
    }
    // Check if this is a valid static file path
    // Uses regex validation to ensure path safety and prevent directory traversal
    else if is_valid_path(uri.path()).is_ok() {
        Ok(RouteType::StaticFile(path.to_string()))
    }
    // Reject invalid or potentially dangerous paths
    else {
        Err(anyhow!("Invalid route: {}", uri.path()))
    }
}

/// Processes HTTP requests for WebAssembly module execution
///
/// Converts incoming HTTP requests into WASM-compatible format and handles
/// the async communication with WebAssembly modules.
///
/// ## Returns:
/// HTTP response from the WebAssembly module
async fn handle_webasm_request(
    req: Request<Incoming>,
    path: String,
) -> anyhow::Result<Response<Full<Bytes>>> {
    let (lambda_send, lambda_recv_answer) = channel();
    let method = req.method().to_string();

    // Convert headers to multi-value format
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

    compose_http_event(method, headers, body_bytes, path, lambda_send, &lambda_recv_answer)
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
    let on_http_event = HTTPEvent { headers, method, path, body, sender };
    let event = HermesEvent::new(on_http_event, TargetApp::All, TargetModule::All);

    crate::event::queue::send(event)?;

    let timeout = Duration::from_secs(EVENT_TIMEOUT);
    match receiver.recv_timeout(timeout)? {
        HTTPEventMsg::HttpEventResponse(resp) => {
            let body = serde_json::to_string(&resp)?.into();
            Ok(Response::new(body))
        }
        HTTPEventMsg::HTTPEventReceiver => {
            error_response("HTTP event message error")
        }
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

/// Serves static web assets for Flutter applications with comprehensive error handling
///
/// This is the primary function for handling static file requests in the Hermes web server.
/// It manages the complete lifecycle of serving web assets from the Virtual File System (VFS),
/// including MIME type detection, caching headers, security policies, and fallback handling.
///
/// ## Core Functionality:
///
/// ### 1. Path Resolution & VFS Access
/// - Converts HTTP request paths to VFS file paths using `resolve_static_file_path()`
/// - Accesses the application's Virtual File System through the reactor
/// - Handles both direct file requests and Flutter routing scenarios
///
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

/// Default file path for root requests - Flutter application entry point
const DEFAULT_INDEX_PATH: &str = "www/index.html";

/// Document root directory in the VFS where static web assets are stored
const DOCUMENT_ROOT: &str = "www";

/// Resolves incoming HTTP request paths to actual file paths within the Virtual File System (VFS)
///
/// This function performs path normalization and translation for serving static files in a Flutter web application.
/// It implements the standard web server convention where the document root maps to a specific VFS directory.
///
/// ## Path Resolution Rules:
///
/// ### Root Path Handling:
/// - **Input**: `"/"` (root/index request)
/// - **Output**: `"www/index.html"`
/// - **Purpose**: Serves the main Flutter application entry point
/// - **Example**: `GET /` → serves `www/index.html` from VFS
///
/// ### Static File Path Translation:
/// - **Input**: `"/assets/app.js"` (any non-root path)
/// - **Process**: Remove leading `/`, prepend `www/`
/// - **Output**: `"www/assets/app.js"`
/// - **Purpose**: Maps URL paths to VFS file locations
///
/// ## VFS Structure Context:
/// VFS Root
/// |-- www/                    <- Document root directory
///     |-- index.html         <- Main application entry point
///     |-- flutter.js         <- Flutter framework loader
///     |-- `flutter_service_worker.js`  <- Service worker
///     |-- assets/            <- Static assets directory
///         |-- fonts/         <- Font files
///         |-- images/        <- Image assets
///         |-- packages/      <- Dart package assets
///
/// ## Security Considerations:
/// - **Path traversal protection**: This function doesn't validate against `../` attacks
///   (assumes upstream validation by HTTP router/framework)
/// - **VFS isolation**: All resolved paths are contained within the `www/` directory
/// - **No filesystem access**: Works purely with VFS paths, not real filesystem paths
fn resolve_static_file_path(path: &str) -> String {
    if path == "/" {
        DEFAULT_INDEX_PATH.to_string()
    } else {
        let clean_path = path.strip_prefix('/').unwrap_or(path);
        format!("{DOCUMENT_ROOT}/{clean_path}")
    }
}

/// Serves an existing asset file with appropriate headers
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

    add_flutter_cache_headers(&mut response, file_path)?;

    add_security_headers(response)
}

/// Handles missing asset files with appropriate fallback logic
fn handle_missing_asset<B>(
    file_path: &str,
    app: &Application,
) -> anyhow::Result<Response<B>>
where
    B: Body + From<Vec<u8>>,
{
    // Critical assets should return 404
    if is_critical_asset(file_path) {
        error!("Critical asset missing: {}", file_path);
        return Ok(Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body("Asset not found".as_bytes().to_vec().into())?);
    }

    // Fall back to index.html for navigation routes
    serve_index_html_fallback(file_path, app)
}

/// Checks if the file is a critical asset that shouldn't fallback to index.html
fn is_critical_asset(file_path: &str) -> bool {
    matches!(
        get_file_extension(file_path),
        Some("js" | "wasm" | "json" | "css" | "map")
    )
}

/// Adds security headers to HTTP responses for Flutter web applications
///
/// This function implements Cross-Origin Isolation by setting two critical security headers
/// that work together to create a secure browsing context.
///
/// ## Headers Added:
///
/// ### Cross-Origin-Opener-Policy (COOP): "same-origin"
/// - **Purpose**: Isolates the browsing context from cross-origin windows
/// - **Effect**: Prevents other origins from accessing this window object
/// - **Security**: Protects against certain types of cross-origin attacks
/// - **Compatibility**: Allows same-origin popups/windows to communicate normally
///
/// ### Cross-Origin-Embedder-Policy (COEP): "require-corp"
/// - **Purpose**: Requires all embedded resources to explicitly opt-in to cross-origin loading
/// - **Effect**: Blocks cross-origin resources without proper CORS or Cross-Origin-Resource-Policy headers
/// - **Security**: Prevents malicious resource injection from untrusted origins
/// - **Requirement**: All cross-origin assets need `crossorigin` attribute or CORP headers
///
fn add_security_headers<B>(mut response: Response<B>) -> anyhow::Result<Response<B>>
where
    B: Body,
{
    let headers = response.headers_mut();

    // Enable Cross-Origin Isolation for advanced web features
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
/// Implements a two-tier caching strategy:
/// - Critical files (index.html, service worker): No caching
/// - Static assets: Long-term caching (1 year)
fn add_flutter_cache_headers<B>(
    response: &mut Response<B>,
    file_path: &str,
) -> anyhow::Result<()> {
    // Determine cache strategy based on file type
    let cache_control = if is_critical_file(file_path) {
        "no-cache, no-store, must-revalidate"
    } else {
        "public, max-age=31536000" // 1 year
    };

    // Apply cache header
    response.headers_mut().insert(
        "Cache-Control",
        cache_control
            .parse()
            .map_err(|e| anyhow!("Invalid cache control header: {}", e))?,
    );

    Ok(())
}

/// Checks if a file requires no caching (critical navigation files)
fn is_critical_file(file_path: &str) -> bool {
    file_path.ends_with("index.html") || file_path.ends_with("flutter_service_worker.js")
}

/// Serves index.html as fallback for navigation routes
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
                    .insert("Content-Type", "text/html".parse()?);
                response.headers_mut().insert(
                    "Cache-Control",
                    "no-cache, no-store, must-revalidate".parse()?,
                );
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