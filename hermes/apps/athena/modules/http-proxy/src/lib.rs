//! # HTTP Proxy Component - Temporary Bridge to External Endpoints
//! 
//! Mirrors HTTP requests to Cat Voices endpoints where we lack native Hermes implementations.
//! Provides a bridge during development while we progressively implement each endpoint natively.
//! 
//! ## Request Flow
//! Client → Gateway (port 5000) → WASM Component → Internal Redirect → External Endpoint
//! 
//! ## Currently Mirrored Endpoints
//! - `/api/gateway/v1/config/frontend` - UI configuration
//! - `/api/gateway/v1/cardano/assets/*` - Blockchain assets (wildcard)
//! - `/api/gateway/v1/rbac/registration` - Authentication
//! - `/api/gateway/v2/document/index` - Document API v2
//! 
//! All HTTP methods (GET, POST, PUT, DELETE) supported automatically.
//! Query parameters and request bodies preserved in redirects.
//! 
//! ## Security
//! Redirects validated against configurable policies:
//! ```bash
//! REDIRECT_ALLOWED_HOSTS="app.dev.projectcatalyst.io"
//! REDIRECT_ALLOWED_PATH_PREFIXES="/api/gateway"  
//! REDIRECT_ALLOWED_SCHEMES="https"
//! ```
//! 
//! ## Testing Parity
//! Test that local responses match production responses:
//! ```bash
//! # Frontend config comparison
//! PROD=$(curl -s "https://app.dev.projectcatalyst.io/api/gateway/v1/config/frontend")
//! LOCAL=$(curl -s -H "Host: app.hermes.local" "http://0.0.0.0:5000/api/gateway/v1/config/frontend" | jq -r '.[2] | implode | fromjson')
//! jq --argjson prod "$PROD" --argjson local "$LOCAL" -n '$prod == $local'
//! 
//! # Cardano assets with query parameters
//! ENDPOINT="/api/gateway/v1/cardano/assets/stake_test1ursne3ndzr4kz8gmhmstu5026erayrnqyj46nqkkfcn0uf?asat=SLOT:95022059"
//! PROD=$(curl -s "https://app.dev.projectcatalyst.io${ENDPOINT}")
//! LOCAL=$(curl -s -H "Host: app.hermes.local" "http://0.0.0.0:5000${ENDPOINT}" | jq -r '.[2] | implode | fromjson')
//! 
//! # RBAC with authentication
//! AUTH="Bearer catid.:1755256930@preprod.cardano/..."
//! PROD=$(curl -s "https://app.dev.projectcatalyst.io/api/gateway/v1/rbac/registration" -H "Authorization: $AUTH")
//! LOCAL=$(curl -s -H "Host: app.hermes.local" -H "Authorization: $AUTH" "http://0.0.0.0:5000/api/gateway/v1/rbac/registration" | jq -r '.[2] | implode | fromjson')
//! ```
//! 
//! ## Migration Strategy
//! As native implementations are developed, remove endpoints from this proxy.
//! Eventually deprecate external redirects when native implementations are complete.

// Allow everything since this is generated code.
#[allow(clippy::all, unused)]
mod hermes;
mod stub;

use crate::hermes::exports::hermes::http_gateway::event::{HttpGatewayResponse};
use crate::hermes::hermes::binary::api::Bstr;
use crate::hermes::exports::hermes::http_gateway::event::HttpResponse;

use std::sync::OnceLock;

/// External Cat Voices host for temporary mirroring
const MIRROR_HOST: &str = "https://app.dev.projectcatalyst.io";

/// Regex patterns for endpoints that should be mirrored to external service
/// 
/// Patterns match:
/// - Frontend config: `/api/gateway/v1/config/frontend`
/// - Cardano assets: `/api/gateway/v1/cardano/assets/...` (any asset ID)
/// - RBAC registration: `/api/gateway/v1/rbac/registration` (with optional query params)
/// - Document v1: `/api/gateway/v1/document`  
/// - Document v2: `/api/gateway/v2/document/index`
const MIRROR_PATTERNS: &[&str] = &[
    r"^/api/gateway/v1/config/frontend$",
    r"^/api/gateway/v1/cardano/assets/.+$",
    r"^/api/gateway/v1/rbac/registration(\?.*)?$",
    r"^/api/gateway/v1/document$",
    r"^/api/gateway/v2/document/index$",
];

/// Regex pattern for static content (handled natively)
const STATIC_PATTERN: &str = r"^/static/.+$";

/// Compiled regex patterns (initialized once at runtime)
static MIRROR_REGEX: OnceLock<Vec<regex::Regex>> = OnceLock::new();
static STATIC_REGEX: OnceLock<regex::Regex> = OnceLock::new();

/// HTTP proxy component providing temporary bridge to external endpoints.
struct HttpProxyComponent;

/// Initialize regex patterns (called once at startup)
fn init_regex() -> &'static Vec<regex::Regex> {
    MIRROR_REGEX.get_or_init(|| {
        MIRROR_PATTERNS
            .iter()
            .map(|pattern| {
                regex::Regex::new(pattern)
                    .unwrap_or_else(|e| panic!("Invalid regex pattern '{}': {}", pattern, e))
            })
            .collect()
    })
}

/// Initialize static content regex pattern
fn init_static_regex() -> &'static regex::Regex {
    STATIC_REGEX.get_or_init(|| {
        regex::Regex::new(STATIC_PATTERN)
            .unwrap_or_else(|e| panic!("Invalid static regex pattern '{}': {}", STATIC_PATTERN, e))
    })
}

/// Determines if a path should be mirrored to external endpoint using regex matching
fn should_mirror(path: &str) -> bool {
    let patterns = init_regex();
    patterns.iter().any(|regex| regex.is_match(path))
}

/// Determines if a path is static content using regex matching
fn is_static_content(path: &str) -> bool {
    let regex = init_static_regex();
    regex.is_match(path)
}

/// Creates a mirrored redirect response
fn create_mirror_redirect(path: &str) -> HttpGatewayResponse {
    log_debug(&format!("Mirroring endpoint to Cat Voices: {}", path));
    HttpGatewayResponse::InternalRedirect(format!("{}{}", MIRROR_HOST, path))
}

/// Creates a static content response (native handling)
fn create_static_response(path: &str) -> HttpGatewayResponse {
    log_debug(&format!("Serving static content natively: {}", path));
    HttpGatewayResponse::Http(HttpResponse {
        code: 200,
        headers: vec![("content-type".to_string(), vec!["text/plain".to_string()])],
        body: Bstr::from(format!("Static file content for: {}", path)),
    })
}

/// Creates a 404 not found response
fn create_not_found_response(method: &str, path: &str) -> HttpGatewayResponse {
    log_warn(&format!("Route not found (no native implementation or mirroring configured): {} {}", method, path));
    HttpGatewayResponse::Http(HttpResponse {
        code: 404,
        headers: vec![("content-type".to_string(), vec!["text/html".to_string()])],
        body: Bstr::from("<html><body><h1>404 - Page Not Found</h1></body></html>"),
    })
}

/// Logs an info message
fn log_info(message: &str) {
    hermes::hermes::logging::api::log(
        hermes::hermes::logging::api::Level::Info,
        Some("http-proxy"),
        None, None, None, None,
        message,
        None,
    );
}

/// Logs a debug message
fn log_debug(message: &str) {
    hermes::hermes::logging::api::log(
        hermes::hermes::logging::api::Level::Debug,
        Some("http-proxy"),
        None, None, None, None,
        message,
        None,
    );
}

/// Logs a warning message
fn log_warn(message: &str) {
    hermes::hermes::logging::api::log(
        hermes::hermes::logging::api::Level::Warn,
        Some("http-proxy"),
        None, None, None, None,
        message,
        None,
    );
}

/// Formats the response type for logging
fn format_response_type(response: &HttpGatewayResponse) -> String {
    match response {
        HttpGatewayResponse::Http(resp) => format!("HTTP {}", resp.code),
        HttpGatewayResponse::InternalRedirect(_) => "REDIRECT (temporary bridge)".to_string(),
    }
}

impl hermes::exports::hermes::http_gateway::event::Guest for HttpProxyComponent {
    /// Routes requests to external endpoints when native implementations don't exist.
    fn reply(
        _body: Vec<u8>,
        _headers: hermes::exports::hermes::http_gateway::event::Headers,
        path: String,
        method: String,
    ) -> Option<HttpGatewayResponse> {
        log_info(&format!("Processing HTTP request: {} {}", method, path));

        let response = if should_mirror(&path) {
            create_mirror_redirect(&path)
        } else if is_static_content(&path) {
            create_static_response(&path)
        } else {
            create_not_found_response(&method, &path)
        };

        log_info(&format!(
            "Request completed: {} {} -> {}", 
            method, 
            path, 
            format_response_type(&response)
        ));

        Some(response)
    }
}

hermes::export!(HttpProxyComponent with_types_in hermes);