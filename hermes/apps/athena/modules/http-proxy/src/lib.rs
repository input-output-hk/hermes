//! HTTP Proxy Module - Configurable Request Router
//!
//! ## Vision
//! This module is designed to be a fully configurable HTTP proxy system that can:
//! - Route requests to different backends based on configurable rules
//! - Support multiple routing strategies (path-based, header-based, etc.)
//! - Handle load balancing and failover scenarios
//! - Provide middleware capabilities for request/response transformation
//! - Offer dynamic configuration updates without restarts
//!
//! ## Current State
//! At present, the module serves as a temporary bridge to external Cat Voices endpoints
//! while native implementations are under development. The current focus is on:
//! - Maintaining API compatibility during the transition period
//! - Ensuring reliable request forwarding to external services
//! - Providing seamless user experience while backend services migrate
//!
//! ## Roadmap
//! As native implementations are completed, this module will evolve into a sophisticated
//! proxy system capable of routing between multiple backends, supporting A/B testing,
//! gradual rollouts, and advanced traffic management scenarios.

use regex::RegexSet;
use std::sync::OnceLock;

use exports::hermes::http_gateway::event::{
    Bstr, Guest as _, Headers, HttpGatewayResponse, HttpResponse,
};

wit_bindgen::generate!({
    world: "hermes:app/hermes",
    path: "../../../wasi/wit",
    inline: "
        package hermes:app;

        world hermes {
            import hermes:logging/api;
            export hermes:http-gateway/event;
            
        }
    ",
    generate_all,
});
export!(HttpProxyComponent);

use hermes::logging::api::{log, Level};

/// What to do when a route pattern matches
#[derive(Debug, Clone, Copy)]
enum RouteAction {
    External, // Forward to Cat Voices
    Static,   // Serve natively
}

/// Compiled patterns for efficient matching
static ROUTE_MATCHER: OnceLock<(RegexSet, Vec<RouteAction>)> = OnceLock::new();

/// External Cat Voices host for temporary external routing
/// TODO: Make this configurable via environment variables or config file
const EXTERNAL_HOST: &str = "https://app.dev.projectcatalyst.io";

/// Route patterns that should be forwarded to external Cat Voices system
/// TODO: Convert to configurable rules engine supporting dynamic pattern updates
const EXTERNAL_ROUTE_PATTERNS: &[&str] = &[
    r"^/api/gateway/v1/config/frontend$",
    r"^/api/gateway/v1/cardano/assets/.+$",
    r"^/api/gateway/v1/rbac/registration.*$",
    r"^/api/gateway/v1/document.*$",
    r"^/api/gateway/v2/document.*$", //  can handle subpaths and query parameters
];

/// Regex pattern for static content (handled natively)
/// TODO: Make static content patterns configurable
const STATIC_PATTERN: &str = r"^/static/.+$";

/// HTTP proxy component providing configurable request routing.
///
/// Currently serves as a temporary bridge to external Cat Voices endpoints
/// while native implementations are developed. The long-term vision is to
/// evolve this into a full-featured configurable proxy supporting:
/// - Dynamic backend selection
/// - Load balancing strategies  
/// - Circuit breakers and health checks
/// - Request/response middleware chains
/// - A/B testing and canary deployments
struct HttpProxyComponent;

/// Initialize all route patterns as a single RegexSet
fn init_route_matcher() -> &'static (RegexSet, Vec<RouteAction>) {
    ROUTE_MATCHER.get_or_init(|| {
        let mut patterns = Vec::new();
        let mut actions = Vec::new();

        // External routes (redirect to Cat Voices)
        for pattern in EXTERNAL_ROUTE_PATTERNS {
            patterns.push(*pattern);
            actions.push(RouteAction::External);
        }

        // Static content (serve natively)
        patterns.push(STATIC_PATTERN);
        actions.push(RouteAction::Static);

        // Compile all patterns together for performance
        let regex_set = RegexSet::new(&patterns).unwrap_or_else(|e| {
            log_warn(&format!("Failed to compile patterns: {}", e));
            RegexSet::empty()
        });

        (regex_set, actions)
    })
}

/// Get the action for a given path
fn get_route_action(path: &str) -> Option<RouteAction> {
    let (regex_set, actions) = init_route_matcher();
    regex_set.matches(path).iter().next().map(|i| actions[i])
}

/// Check if path should route externally
fn should_route_externally(path: &str) -> bool {
    matches!(get_route_action(path), Some(RouteAction::External))
}

/// Check if path is static content
fn is_static_content(path: &str) -> bool {
    matches!(get_route_action(path), Some(RouteAction::Static))
}

/// Creates an external route redirect response
/// Currently redirects to Cat Voices - will become configurable backend selection
fn create_external_redirect(path: &str) -> HttpGatewayResponse {
    log_debug(&format!("Routing externally to Cat Voices: {}", path));
    HttpGatewayResponse::InternalRedirect(format!("{}{}", EXTERNAL_HOST, path))
}

/// Creates a static content response (native handling)
/// TODO: Integrate with configurable static content serving middleware
fn create_static_response(path: &str) -> HttpGatewayResponse {
    log_debug(&format!("Serving static content natively: {}", path));
    HttpGatewayResponse::Http(HttpResponse {
        code: 200,
        headers: vec![("content-type".to_string(), vec!["text/plain".to_string()])],
        body: Bstr::from(format!("Static file content for: {}", path)),
    })
}

/// Creates a 404 not found response
/// TODO: Make error responses configurable (custom error pages, etc.)
fn create_not_found_response(
    method: &str,
    path: &str,
) -> HttpGatewayResponse {
    log_warn(&format!(
        "Route not found (no native implementation or external routing configured): {} {}",
        method, path
    ));
    HttpGatewayResponse::Http(HttpResponse {
        code: 404,
        headers: vec![("content-type".to_string(), vec!["text/html".to_string()])],
        body: Bstr::from("<html><body><h1>404 - Page Not Found</h1></body></html>"),
    })
}

/// Logs an info message
fn log_info(message: &str) {
    log(
        Level::Info,
        Some("http-proxy"),
        None,
        None,
        None,
        None,
        message,
        None,
    );
}

/// Logs a debug message
fn log_debug(message: &str) {
    log(
        Level::Debug,
        Some("http-proxy"),
        None,
        None,
        None,
        None,
        message,
        None,
    );
}

/// Logs a warning message
fn log_warn(message: &str) {
    log(
        Level::Warn,
        Some("http-proxy"),
        None,
        None,
        None,
        None,
        message,
        None,
    );
}

/// Formats the response type for logging
fn format_response_type(response: &HttpGatewayResponse) -> String {
    match response {
        HttpGatewayResponse::Http(resp) => format!("HTTP {}", resp.code),
        HttpGatewayResponse::InternalRedirect(_) => {
            "EXTERNAL_REDIRECT (temporary bridge)".to_string()
        },
    }
}

impl exports::hermes::http_gateway::event::Guest for HttpProxyComponent {
    /// Routes HTTP requests through configurable proxy logic.
    ///
    /// Current implementation provides temporary bridging to external Cat Voices
    /// endpoints while native implementations are developed. Future versions will
    /// support sophisticated routing rules, backend selection, and middleware chains.
    fn reply(
        _body: Vec<u8>,
        _headers: Headers,
        path: String,
        method: String,
    ) -> Option<HttpGatewayResponse> {
        log_info(&format!("Processing HTTP request: {} {}", method, path));

        let response = if should_route_externally(&path) {
            create_external_redirect(&path)
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
