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
//! - `/api/gateway/v1/document` - Document API v1
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
//! Eventually deprecate this component when all logic is native.

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

/// Document endpoint (temporary mirror until native implementation)
const DOCUMENT_ENDPOINT: &str = "/api/gateway/v1/document";

/// Document v2 endpoint (temporary mirror until native implementation)
const DOCUMENT_V2_ENDPOINT: &str = "/api/gateway/v2/document/index";

/// Static content endpoint prefix (handled natively - no mirroring)
const STATIC_ENDPOINT_PREFIX: &str = "/static/";

/// External Cat Voices host for temporary mirroring
const MIRROR_HOST: &str = "https://app.dev.projectcatalyst.io";

/// HTTP proxy component providing temporary bridge to external endpoints.
struct HttpProxyComponent;

impl hermes::exports::hermes::http_gateway::event::Guest for HttpProxyComponent {
    /// Routes requests to external endpoints when native implementations don't exist.
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
            DOCUMENT_ENDPOINT => {
                hermes::hermes::logging::api::log(
                    hermes::hermes::logging::api::Level::Debug,
                    Some("http-proxy"),
                    None,
                    None,
                    None,
                    None,
                    "Temporarily mirroring document v1 endpoint to Cat Voices",
                    None,
                );
                HttpGatewayResponse::InternalRedirect(format!("{}{}", MIRROR_HOST, DOCUMENT_ENDPOINT))
            },
            DOCUMENT_V2_ENDPOINT => {
                hermes::hermes::logging::api::log(
                    hermes::hermes::logging::api::Level::Debug,
                    Some("http-proxy"),
                    None,
                    None,
                    None,
                    None,
                    "Temporarily mirroring document v2 endpoint to Cat Voices",
                    None,
                );
                HttpGatewayResponse::InternalRedirect(format!("{}{}", MIRROR_HOST, DOCUMENT_V2_ENDPOINT))
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