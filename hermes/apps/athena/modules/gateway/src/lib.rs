//! Catalyst Gateway API

mod api;
mod common;
mod rbac;
mod settings;
mod utilities;
mod utils;

use regex::RegexSet;
use std::sync::OnceLock;

use exports::hermes::http_gateway::event::{Bstr, Headers, HttpGatewayResponse, HttpResponse};

wit_bindgen::generate!({
    world: "hermes:app/hermes",
    path: "../../../../../wasm/wasi/wit",
    inline: "
        package hermes:app;

        world hermes {
            include wasi:cli/imports@0.2.6;
            import hermes:logging/api;
            export hermes:http-gateway/event;

        }
    ",
    generate_all,
});
export!(CatGatewayAPI);

use hermes::logging::api::{log, Level};

use crate::{
    api::cardano::staking::{self, Api},
    common::{auth::none::NoAuthorization, types::cardano::cip19_stake_address::Cip19StakeAddress},
};

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

const STAKE_ROUTE: &str = "/api/gateway/v1/cardano/assets/:stake_address";

/// Route patterns that should be forwarded to external Cat Voices system
/// TODO: Convert to configurable rules engine supporting dynamic pattern updates
const EXTERNAL_ROUTE_PATTERNS: &[&str] = &[
    // // RBAC
    // "v1/rbac/registration/",
    // "v2/rbac/registration/",
    // // Cardano
    // "v1/cardano/registration/cip36",
    "v1/cardano/assets/:stake_address",
    // // Config
    // "v1/config/frontend",
    // // Document
    // "v1/document:document_id",
    // "v1/document",
    // "v1/document/index",
    // "v2/document/index",
    // // Health
    // "v1/health/started",
    // "v1/health/ready",
    // "v1/health/live",
    // "v1/health/inspection",
    // // Upload
    // "upload",
    // "upload_stream",
];

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
struct CatGatewayAPI;

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

impl exports::hermes::http_gateway::event::Guest for CatGatewayAPI {
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

        let response = match path.to_lowercase().as_str() {
            STAKE_ROUTE => {
                let response = Api.staked_ada_get(
                    Cip19StakeAddress::try_from("asd").unwrap(),
                    None,
                    None,
                    common::auth::none_or_rbac::NoneOrRBAC::None(NoAuthorization),
                );
                todo!("transform response")
            },
            _ => create_not_found_response(&method, &path),
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
