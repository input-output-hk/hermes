//! # HTTP Proxy Component - Hermes WASM Module Example
//! 
//! **This is a toy example** showing basic HTTP routing in Hermes. Real applications
//! would add authentication, error handling, configuration, and security features.
//! 
//! ## Key Patterns Demonstrated
//! - HTTP request routing with pattern matching
//! - Direct HTTP responses vs internal redirects  
//! - Structured logging with appropriate levels
//! - WASM component structure for Hermes
//! 
//! ## Production Extensions
//! Real applications could build on this to create:
//! - API gateways routing to microservices
//! - Load balancers with backend selection
//! - Content management systems
//! - Authentication/authorization layers

// Allow everything since this is generated code.
#[allow(clippy::all, unused)]
mod hermes;
mod stub;

use crate::hermes::exports::hermes::http_gateway::event::{HttpGatewayResponse};
use crate::hermes::hermes::binary::api::Bstr;
use crate::hermes::exports::hermes::http_gateway::event::HttpResponse;

/// Simple HTTP proxy component for demonstration purposes.
struct HttpProxyComponent;

impl hermes::exports::hermes::http_gateway::event::Guest for HttpProxyComponent {
    /// Handle HTTP requests and return responses or redirects.
    /// 
    /// Production systems would add: request validation, authentication,
    /// caching, error handling, and security headers.
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
            "/api" | "/api/index" => {
                hermes::hermes::logging::api::log(
                    hermes::hermes::logging::api::Level::Debug,
                    Some("http-proxy"),
                    None,
                    None,
                    None,
                    None,
                    "Serving homepage content",
                    None,
                );
                HttpGatewayResponse::Http(HttpResponse {
                    code: 200,
                    headers: vec![("content-type".to_string(), vec!["text/html".to_string()])],
                    body: Bstr::from("<html><body><h1>Welcome to the homepage</h1></body></html>"),
                })
            },
            "/api/dashboard" => {
                hermes::hermes::logging::api::log(
                    hermes::hermes::logging::api::Level::Debug,
                    Some("http-proxy"),
                    None,
                    None,
                    None,
                    None,
                    "Redirecting to external API: https://catfact.ninja/fact",
                    None,
                );
                HttpGatewayResponse::InternalRedirect("https://catfact.ninja/fact".to_string())
            },
            path if path.starts_with("/static/") => {
                hermes::hermes::logging::api::log(
                    hermes::hermes::logging::api::Level::Debug,
                    Some("http-proxy"),
                    None,
                    None,
                    None,
                    None,
                    format!("Serving static content for: {}", path).as_str(),
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
                    format!("Route not found: {} {}", method, path).as_str(),
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
                    HttpGatewayResponse::InternalRedirect(_) => "REDIRECT".to_string(),
                }
            ).as_str(),
            None,
        );

        Some(response)
    }
}

hermes::export!(HttpProxyComponent with_types_in hermes);