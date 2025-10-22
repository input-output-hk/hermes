//! HTTP Proxy Module - **TEMPORARY** External Bridge
//!
//! ⚠️ **DEPRECATION NOTICE**: This module is a **temporary solution** that will be
//! **deprecated** once native WASM implementations are completed.
//!
//! ## Purpose
//! This module serves as a **temporary bridge** to external Cat Voices endpoints while
//! native WASM modules are under development. It provides:
//! - External redirects to `https://app.dev.projectcatalyst.io`
//! - Temporary API compatibility during migration
//! - Seamless user experience while native services are built
//!
//! ## Migration Strategy
//! The HTTP gateway's subscription system enables seamless migration:
//!
//! **Current (Temporary)**:
//! ```
//! /api/gateway/v1/config → http_proxy module → External redirect
//! ```
//!
//! **Future (Native)**:
//! ```
//! /api/gateway/v1/config → frontend_config_native module → Direct implementation
//! ```
//!
//! Only the `module_id` in `endpoints.json` needs updating - no gateway changes required.
//!
//! ## Deprecation Timeline
//! This module will be removed once all the following native modules are implemented:
//! - `frontend_config_native` (replaces `/api/gateway/v1/config/frontend`)
//! - `cardano_assets_native` (replaces `/api/gateway/v1/cardano/assets/*`)
//! - `rbac_native` (replaces `/api/gateway/v1/rbac/registration*`)
//! - `document_service_native` (replaces `/api/gateway/v*/document*`)
//! - `static_file_native` (replaces `/static/*` if needed)

use std::sync::OnceLock;

use hermes::http_gateway::api::{Bstr, Headers, HttpGatewayResponse, HttpResponse};
use regex::RegexSet;
use shared::utils::log::{self, debug, info, warn};

shared::bindings_generate!({
    world: "hermes:app/hermes",
    path: "../../../../../wasm/wasi/wit",
    inline: "
        package hermes:app;

        world hermes {
            include wasi:cli/imports@0.2.6;
            import hermes:logging/api;
            import hermes:http-gateway/api;

            export hermes:http-gateway/event;
        }
    ",
    share: ["hermes:logging"],
});
export!(HttpProxyComponent);

/// What to do when a route pattern matches
#[derive(Debug, Clone, Copy)]
enum RouteAction {
    External, // Forward to Cat Voices
    Static,   // Serve natively
}

/// Compiled patterns for efficient matching
static ROUTE_MATCHER: OnceLock<(RegexSet, Vec<RouteAction>)> = OnceLock::new();

/// External Cat Voices host for **temporary** external routing
/// ⚠️ TEMPORARY: This will be removed when native modules are ready
const EXTERNAL_HOST: &str = "https://app.dev.projectcatalyst.io";

/// Route patterns for **temporary** external forwarding to Cat Voices system
/// ⚠️ DEPRECATED: These patterns will be replaced by native WASM modules:
/// - `/api/gateway/v1/config/frontend` → `frontend_config_native` module
/// - `/api/gateway/v1/cardano/assets/*` → `cardano_assets_native` module
/// - `/api/gateway/v1/rbac/registration*` → `rbac_native` module
/// - `/api/gateway/v*/document*` → `document_service_native` module
const EXTERNAL_ROUTE_PATTERNS: &[&str] = &[
    r"^/api/gateway/v1/config/frontend$",
    r"^/api/gateway/v1/cardano/assets/.+$",
    r"^/api/gateway/v1/rbac/registration.*$",
    r"^/api/gateway/v1/document.*$",
    r"^/api/gateway/v2/document.*$",
];

/// Regex pattern for static content (handled natively)
/// ⚠️ TEMPORARY: May be removed if static content remains in HTTP gateway
const STATIC_PATTERN: &str = r"^/static/.+$";

/// **TEMPORARY** HTTP proxy component for external bridging.
///
/// ⚠️ **This component will be deprecated** once native WASM modules are completed.
///
/// Currently provides temporary bridging to external Cat Voices endpoints during
/// the migration from external dependencies to native implementations. This allows
/// the system to maintain functionality while native modules are developed.
///
/// **This is NOT intended to be a permanent proxy solution** - it's specifically
/// designed as a migration bridge that will be removed.
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
            warn!(error:err = e; "Failed to compile patterns");
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
/// ⚠️ TEMPORARY: Redirects to Cat Voices - will be replaced by native modules
fn create_external_redirect(path: &str) -> HttpGatewayResponse {
    debug!(path; "Routing externally to Cat Voices");
    HttpGatewayResponse::InternalRedirect(format!("{}{}", EXTERNAL_HOST, path))
}

/// Creates a static content response (native handling)
/// ⚠️ TEMPORARY: May be removed if static content stays in HTTP gateway
fn create_static_response(path: &str) -> HttpGatewayResponse {
    debug!(path; "Serving static content natively");
    HttpGatewayResponse::Http(HttpResponse {
        code: 200,
        headers: vec![("content-type".to_string(), vec!["text/plain".to_string()])],
        body: Bstr::from(format!("Static file content for: {}", path)),
    })
}

/// Creates a 404 not found response
/// ⚠️ TEMPORARY: Simple 404 response - will be removed with this module
fn create_not_found_response(
    method: &str,
    path: &str,
) -> HttpGatewayResponse {
    warn!(
        method,
        path;
        "Route not found (no native implementation or external routing configured)",
    );
    HttpGatewayResponse::Http(HttpResponse {
        code: 404,
        headers: vec![("content-type".to_string(), vec!["text/html".to_string()])],
        body: Bstr::from("<html><body><h1>404 - Page Not Found</h1></body></html>"),
    })
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
    /// Routes HTTP requests through **temporary** proxy logic.
    ///
    /// ⚠️ **TEMPORARY IMPLEMENTATION**: This method provides bridging to external
    /// Cat Voices endpoints during migration to native WASM modules. Once native
    /// implementations are complete, this entire module will be deprecated and
    /// removed.
    ///
    /// The routing behavior is intentionally simple since this is not a permanent
    /// solution - it's designed to be replaced, not enhanced.
    fn reply(
        _body: Vec<u8>,
        _headers: Headers,
        path: String,
        method: String,
    ) -> Option<HttpGatewayResponse> {
        log::init(log::LevelFilter::Trace);

        info!("Processing HTTP request: {} {}", method, path);

        let response = if should_route_externally(&path) {
            create_external_redirect(&path)
        } else if is_static_content(&path) {
            create_static_response(&path)
        } else {
            create_not_found_response(&method, &path)
        };

        info!(
            method,
            path,
            response = format_response_type(&response);
            "Request completed",
        );

        Some(response)
    }
}
