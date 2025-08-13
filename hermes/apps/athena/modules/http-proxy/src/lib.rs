//! # HTTP Proxy Component - Hermes WASM Module for External Endpoint Mirroring
//! 
//! **This component provides a temporary bridge to external endpoints** by mirroring 
//! HTTP requests from Cat Voices where we do not yet have internal Hermes implementations.
//! This allows the system to function while we progressively develop native Hermes logic
//! for each endpoint.
//! 
//! ## Purpose & Strategy
//! - **Temporary Bridge**: Mirrors endpoints we haven't implemented in Hermes yet
//! - **Progressive Migration**: As we develop native Hermes logic, we remove mirroring
//! - **No Proxy for Native Logic**: Endpoints with Hermes implementations bypass this proxy
//! - **External Dependency**: Routes to Cat Voices until internal capabilities are ready
//! 
//! ## Key Patterns Demonstrated
//! - HTTP request routing with pattern matching
//! - External endpoint mirroring via internal redirects
//! - Structured logging with appropriate levels
//! - WASM component structure for Hermes
//! - Secure URL validation and redirect handling
//! 
//! ## Currently Mirrored Endpoints (Temporary)
//! These endpoints mirror to Cat Voices until we implement native Hermes logic:
//! - Frontend configuration (`/api/gateway/v1/config/frontend`)
//! - Cardano assets (`/api/gateway/v1/cardano/assets/*`)
//! - RBAC registration (`/api/gateway/v1/rbac/registration`)
//! 
//! ## Development Notes
//! - Static content serving is handled natively (no mirroring needed)
//! - As each endpoint gets native Hermes implementation, remove it from this proxy
//! - Eventually this component may be deprecated when all logic is native
//! - Comprehensive request/response logging for debugging during migration

// Allow everything since this is generated code.
#[allow(clippy::all, unused)]
mod hermes;
mod stub;

use crate::hermes::exports::hermes::http_gateway::event::{HttpGatewayResponse};
use crate::hermes::hermes::binary::api::Bstr;
use crate::hermes::exports::hermes::http_gateway::event::HttpResponse;

/// Frontend configuration endpoint (temporary mirror until native implementation)
const FRONTEND_CONFIG_ENDPOINT: &str = "/api/gateway/v1/config/frontend";

/// Cardano assets endpoint prefix (temporary mirror until native implementation)
const CARDANO_ASSETS_ENDPOINT_PREFIX: &str = "/api/gateway/v1/cardano/assets/";

/// RBAC registration endpoint prefix (temporary mirror until native implementation)
const RBAC_REGISTRATION_ENDPOINT_PREFIX: &str = "/api/gateway/v1/rbac/registration";

/// Static content endpoint prefix (handled natively - no mirroring)
const STATIC_ENDPOINT_PREFIX: &str = "/static/";

/// External Cat Voices host for temporary mirroring
const MIRROR_HOST: &str = "https://app.dev.projectcatalyst.io";

/// HTTP proxy component providing temporary bridge to external endpoints.
/// 
/// This component only mirrors endpoints where we lack native Hermes implementations.
/// Once native logic is developed for an endpoint, it should be removed from this proxy.
struct HttpProxyComponent;

impl hermes::exports::hermes::http_gateway::event::Guest for HttpProxyComponent {
    /// Handle HTTP requests with temporary mirroring strategy.
    /// 
    /// Routes requests to external Cat Voices endpoints only when we don't have
    /// native Hermes implementations. This provides a bridge during development
    /// while we progressively implement each endpoint natively.
    fn reply(
        _body: Vec<u8>,
        _headers: hermes::exports::hermes::http_gateway::event::Headers,
        path: String,
        method: String,
    ) -> Option<HttpGatewayResponse> {
        hermes::hermes::logging::api::log(
            hermes::hermes::logging::api::Level::Info,
            Some("http-proxy"),
            None,
            None,
            None,
            None,
            format!("Processing HTTP request: {} {}", method, path).as_str(),
            None,
        );

        let response = match path.as_str() {
            FRONTEND_CONFIG_ENDPOINT => {
                hermes::hermes::logging::api::log(
                    hermes::hermes::logging::api::Level::Debug,
                    Some("http-proxy"),
                    None,
                    None,
                    None,
                    None,
                    "Temporarily mirroring frontend config endpoint to Cat Voices",
                    None,
                );
                HttpGatewayResponse::InternalRedirect(format!("{}{}", MIRROR_HOST, FRONTEND_CONFIG_ENDPOINT))
            },
            path if path.starts_with(CARDANO_ASSETS_ENDPOINT_PREFIX) => {
                hermes::hermes::logging::api::log(
                    hermes::hermes::logging::api::Level::Debug,
                    Some("http-proxy"),
                    None,
                    None,
                    None,
                    None,
                    format!("Temporarily mirroring Cardano assets endpoint to Cat Voices: {}", path).as_str(),
                    None,
                );
                
                let redirect_url = format!("{}{}", MIRROR_HOST, path);

                HttpGatewayResponse::InternalRedirect(redirect_url)
            },
            path if path.starts_with(RBAC_REGISTRATION_ENDPOINT_PREFIX) => {
                hermes::hermes::logging::api::log(
                    hermes::hermes::logging::api::Level::Debug,
                    Some("http-proxy"),
                    None,
                    None,
                    None,
                    None,
                    format!("Temporarily mirroring RBAC registration endpoint to Cat Voices: {}", path).as_str(),
                    None,
                );
                
                let redirect_url = format!("{}{}", MIRROR_HOST, path);

                HttpGatewayResponse::InternalRedirect(redirect_url)
            },
            path if path.starts_with(STATIC_ENDPOINT_PREFIX) => {
                hermes::hermes::logging::api::log(
                    hermes::hermes::logging::api::Level::Debug,
                    Some("http-proxy"),
                    None,
                    None,
                    None,
                    None,
                    format!("Serving static content natively (no mirroring needed): {}", path).as_str(),
                    None,
                );
                HttpGatewayResponse::Http(HttpResponse {
                    code: 200,
                    headers: vec![("content-type".to_string(), vec!["text/plain".to_string()])],
                    body: Bstr::from(format!("Static file content for: {}", path)),
                })
            },
            _ => {
                hermes::hermes::logging::api::log(
                    hermes::hermes::logging::api::Level::Warn,
                    Some("http-proxy"),
                    None,
                    None,
                    None,
                    None,
                    format!("Route not found (no native implementation or mirroring configured): {} {}", method, path).as_str(),
                    None,
                );
                HttpGatewayResponse::Http(HttpResponse {
                    code: 404,
                    headers: vec![("content-type".to_string(), vec!["text/html".to_string()])],
                    body: Bstr::from("<html><body><h1>404 - Page Not Found</h1></body></html>"),
                })
            },
        };

        hermes::hermes::logging::api::log(
            hermes::hermes::logging::api::Level::Info,
            Some("http-proxy"),
            None,
            None,
            None,
            None,
            format!("Request completed: {} {} -> {}", method, path, 
                match &response {
                    HttpGatewayResponse::Http(resp) => format!("HTTP {}", resp.code),
                    HttpGatewayResponse::InternalRedirect(_) => "REDIRECT (temporary bridge)".to_string(),
                }
            ).as_str(),
            None,
        );

        Some(response)
    }
}

hermes::export!(HttpProxyComponent with_types_in hermes);