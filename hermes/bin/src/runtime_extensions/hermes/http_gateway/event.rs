//! HTTP Gateway event handling with secure internal redirects.
//!
//! Processes HTTP requests through WASM modules using MPSC channels.
//! Validates redirect URLs against configurable security policies.

use std::collections::HashSet;
use std::env;
use std::sync::mpsc::Sender;
use tracing::error;

use hyper::{self, body::Bytes};
use serde::{Deserialize, Serialize};
use url::Url;

use crate::event::HermesEventPayload;
use crate::runtime_extensions::bindings::exports::hermes::http_gateway::event::HttpGatewayResponse;

// ============================================================================
// Type Aliases
// ============================================================================

/// HTTP status code (200, 404, 500, etc.)
type Code = u16;

/// Headers as key-value pairs, supporting multiple values per key
pub type HeadersKV = Vec<(String, Vec<String>)>;

/// URL path string
type Path = String;

/// HTTP method string (GET, POST, etc.)
type Method = String;

/// Request/response body as bytes
type Body = Vec<u8>;

// ============================================================================
// Message Types
// ============================================================================

/// MPSC message types for HTTP event communication
#[derive(Serialize, Deserialize, Debug)]
pub(crate) enum HTTPEventMsg {
    /// Receiver acknowledgment
    HTTPEventReceiver,
    /// Event response: (status_code, headers, body)
    HttpEventResponse((Code, HeadersKV, Body)),
}

// ============================================================================
// HTTP Event Structure
// ============================================================================

/// HTTP request event to be processed by WASM modules
pub(crate) struct HTTPEvent {
    pub(crate) headers: HeadersKV,
    pub(crate) method: Method,
    pub(crate) path: Path,
    pub(crate) body: Bytes,
    /// Channel to send response back to client
    pub(crate) sender: Sender<HTTPEventMsg>,
}

// ============================================================================
// Redirect Configuration
// ============================================================================

/// Security configuration for validating internal redirects
#[derive(Debug, Clone)]
pub struct RedirectConfig {
    /// Allowed URL schemes (e.g., "https")
    pub allowed_schemes: HashSet<String>,
    /// Allowed hostnames (e.g., "api.example.com")
    pub allowed_hosts: HashSet<String>,
    /// Allowed path prefixes (e.g., "/api/v1")
    pub allowed_path_prefixes: Vec<String>,
}

impl Default for RedirectConfig {
    fn default() -> Self {
        Self {
            allowed_schemes: ["https"].iter().map(|s| s.to_string()).collect(),
            allowed_hosts: ["app.dev.projectcatalyst.io"]
                .iter()
                .map(|s| s.to_string())
                .collect(),
            allowed_path_prefixes: vec!["/api/gateway".to_string()],
        }
    }
}

impl RedirectConfig {
    /// Load from environment variables:
    /// - REDIRECT_ALLOWED_SCHEMES
    /// - REDIRECT_ALLOWED_HOSTS  
    /// - REDIRECT_ALLOWED_PATH_PREFIXES
    pub fn from_env() -> Self {
        let allowed_schemes = Self::parse_env_list("REDIRECT_ALLOWED_SCHEMES", "https");
        let allowed_hosts =
            Self::parse_env_list("REDIRECT_ALLOWED_HOSTS", "app.dev.projectcatalyst.io");
        let allowed_path_prefixes =
            Self::parse_env_list("REDIRECT_ALLOWED_PATH_PREFIXES", "/api/gateway")
                .into_iter()
                .collect();

        Self {
            allowed_schemes,
            allowed_hosts,
            allowed_path_prefixes,
        }
    }

    /// Parse comma-separated environment variable
    fn parse_env_list(env_var: &str, default: &str) -> HashSet<String> {
        env::var(env_var)
            .unwrap_or_else(|_| default.to_string())
            .split(',')
            .map(|s| s.trim().to_string())
            .collect()
    }
}

// ============================================================================
// Redirect Validation
// ============================================================================

/// Validates redirect URL against security policies
fn validate_redirect_location(location: &str, config: &RedirectConfig) -> anyhow::Result<()> {
    let url =
        Url::parse(location).map_err(|_| anyhow::anyhow!("Invalid redirect URL: {}", location))?;

    validate_scheme(&url, config)?;
    validate_host(&url, config)?;
    validate_path(&url, config)?;

    Ok(())
}

/// Validates URL scheme against allowed schemes
fn validate_scheme(url: &Url, config: &RedirectConfig) -> anyhow::Result<()> {
    if !config.allowed_schemes.contains(url.scheme()) {
        return Err(anyhow::anyhow!(
            "Redirect scheme '{}' not allowed. Allowed schemes: {:?}",
            url.scheme(),
            config.allowed_schemes
        ));
    }
    Ok(())
}

/// Validates URL host against allowed hosts
fn validate_host(url: &Url, config: &RedirectConfig) -> anyhow::Result<()> {
    let host = url
        .host_str()
        .ok_or_else(|| anyhow::anyhow!("No host in redirect URL"))?;

    if !config.allowed_hosts.contains(host) {
        return Err(anyhow::anyhow!(
            "Redirect host '{}' not allowed. Allowed hosts: {:?}",
            host,
            config.allowed_hosts
        ));
    }
    Ok(())
}

/// Validates URL path against allowed prefixes
fn validate_path(url: &Url, config: &RedirectConfig) -> anyhow::Result<()> {
    let path = url.path();
    let path_allowed = config
        .allowed_path_prefixes
        .iter()
        .any(|prefix| path.starts_with(prefix));

    if !path_allowed {
        return Err(anyhow::anyhow!(
            "Redirect path '{}' not allowed. Allowed prefixes: {:?}",
            path,
            config.allowed_path_prefixes
        ));
    }
    Ok(())
}

// ============================================================================
// HTTP Event Processing
// ============================================================================

impl HermesEventPayload for HTTPEvent {
    fn event_name(&self) -> &'static str {
        "http-event"
    }

    fn execute(&self, module: &mut crate::wasm::module::ModuleInstance) -> anyhow::Result<()> {
        let event_response = module.instance.hermes_http_gateway_event().call_reply(
            &mut module.store,
            &self.body.as_ref().to_vec(),
            &self.headers,
            &self.path,
            &self.method,
        )?;

        match event_response {
            Some(HttpGatewayResponse::Http(resp)) => {
                self.send_http_response(resp.code, resp.headers, resp.body)
            },
            Some(HttpGatewayResponse::InternalRedirect(location)) => {
                self.handle_internal_redirect(location)
            },
            None => Ok(()),
        }
    }
}

impl HTTPEvent {
    /// Send HTTP response back to client via MPSC channel
    fn send_http_response(&self, code: Code, headers: HeadersKV, body: Body) -> anyhow::Result<()> {
        Ok(self
            .sender
            .send(HTTPEventMsg::HttpEventResponse((code, headers, body)))?)
    }

    /// Handle internal redirect with security validation
    fn handle_internal_redirect(&self, location: String) -> anyhow::Result<()> {
        let config = RedirectConfig::from_env();

        if let Err(e) = validate_redirect_location(&location, &config) {
            error!("Invalid redirect location: {}", e);
            return self.send_error_response(403, "Forbidden: Invalid redirect location");
        }

        self.spawn_redirect_request(location)
    }

    /// Spawn background thread for redirect request
    fn spawn_redirect_request(&self, location: String) -> anyhow::Result<()> {
        let headers = self.headers.clone();
        let method = self.method.clone();
        let body = self.body.clone();
        let sender = self.sender.clone();

        std::thread::spawn(move || {
            if let Err(e) = Self::execute_redirect_request(location, headers, method, body, sender)
            {
                error!("Redirect request failed: {:?}", e);
            }
        });

        Ok(())
    }

    /// Execute HTTP redirect request
    fn execute_redirect_request(
        location: String, headers: HeadersKV, method: Method, body: Bytes,
        sender: Sender<HTTPEventMsg>,
    ) -> anyhow::Result<()> {
        let client = std::sync::Arc::new(reqwest::blocking::Client::new());
        let request = Self::build_request(&client, &location, &headers, &method, &body)?;

        match request.send() {
            Ok(response) => Self::process_response(response, sender),
            Err(e) => {
                error!("HTTP request failed: {:?}", e);
                Self::send_error_via_sender(sender, 500, "Internal Server Error")
            },
        }
    }

    /// Build HTTP request for redirect (excludes Host header)
    fn build_request(
        client: &reqwest::blocking::Client, location: &str, headers: &HeadersKV, method: &str,
        body: &Bytes,
    ) -> anyhow::Result<reqwest::blocking::RequestBuilder> {
        let mut request = client.request(
            reqwest::Method::from_bytes(method.as_bytes()).unwrap_or(reqwest::Method::GET),
            location,
        );

        // Add headers from original request, excluding Host
        for (key, values) in headers {
            if key.to_lowercase() != "host" {
                for value in values {
                    request = request.header(key, value);
                }
            }
        }

        // Add body if present
        if !body.is_empty() {
            request = request.body(body.to_vec());
        }

        Ok(request)
    }

    /// Process HTTP response and forward to client
    fn process_response(
        response: reqwest::blocking::Response, sender: Sender<HTTPEventMsg>,
    ) -> anyhow::Result<()> {
        let status_code = response.status().as_u16();
        let headers: HeadersKV = response
            .headers()
            .iter()
            .map(|(name, value)| {
                (
                    name.to_string(),
                    vec![value.to_str().unwrap_or("").to_string()],
                )
            })
            .collect();

        match response.bytes() {
            Ok(body) => {
                sender.send(HTTPEventMsg::HttpEventResponse((
                    status_code,
                    headers,
                    body.to_vec(),
                )))?;
            },
            Err(e) => {
                error!("Failed to read response body: {:?}", e);
                Self::send_error_via_sender(sender, 500, "Internal Server Error")?;
            },
        }
        Ok(())
    }

    /// Send error response to client
    fn send_error_response(&self, code: Code, message: &str) -> anyhow::Result<()> {
        self.sender.send(HTTPEventMsg::HttpEventResponse((
            code,
            vec![],
            message.as_bytes().to_vec(),
        )))?;
        Ok(())
    }

    /// Helper to send error via sender channel
    fn send_error_via_sender(
        sender: Sender<HTTPEventMsg>, code: Code, message: &str,
    ) -> anyhow::Result<()> {
        sender.send(HTTPEventMsg::HttpEventResponse((
            code,
            vec![],
            message.as_bytes().to_vec(),
        )))?;
        Ok(())
    }
}
