#![allow(dead_code, missing_docs)]
//! Catalyst Gateway API

mod api;
mod config;
mod error;

use std::sync::OnceLock;

use hermes::http_gateway::api::{Bstr, Headers, HttpGatewayResponse, HttpResponse};

shared::bindings_generate!({
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
    share: ["hermes:logging"],
});
export!(CatGatewayAPI);

use http::{header::CONTENT_TYPE, StatusCode};
use mime::{APPLICATION_JSON, TEXT_HTML};
use regex::Regex;
use shared::utils::{
    common::{
        auth::none::NoAuthorization,
        responses::{ErrorResponses, WithErrorResponses},
        types::cardano::cip19_stake_address::Cip19StakeAddress,
    },
    log,
};

use crate::{
    api::{staked_ada_get, types::Responses, GetStakedAdaRequest},
    config::messages,
    error::{Result, StakedAdaError},
};

/// Staked ada route regex.
static STAKE_ROUTE_REGEX: OnceLock<Regex> = OnceLock::new();

/// Initializes staked ada route regex.
#[allow(clippy::expect_used)]
fn stake_route_regex() -> &'static Regex {
    STAKE_ROUTE_REGEX.get_or_init(|| {
        Regex::new(crate::config::STAKE_ROUTE_PATTERN)
            .expect("Hardcoded regex pattern should be valid")
    })
}

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

/// Creates a 404 not found response
/// TODO: Make error responses configurable (custom error pages, etc.)
fn create_not_found_response(
    method: &str,
    path: &str,
) -> HttpGatewayResponse {
    const FUNCTION_NAME: &str = "create_not_found_response";
    log::warn!(
        "Route not found (no native implementation or external routing configured): {method} {path}",
    );
    HttpGatewayResponse::Http(HttpResponse {
        code: StatusCode::NOT_FOUND.as_u16(),
        headers: vec![(CONTENT_TYPE.to_string(), vec![TEXT_HTML.to_string()])],
        body: Bstr::from(format!(
            "<html><body><h1>{}</h1></body></html>",
            messages::PAGE_NOT_FOUND
        )),
    })
}

/// Creates a 400 Bad Request response
/// TODO: Make error responses configurable (custom error pages, etc.)
fn create_bad_request_response(
    method: &str,
    path: &str,
) -> HttpGatewayResponse {
    const FUNCTION_NAME: &str = "create_bad_request_response";
    log::warn!("Invalid route: {method} {path}",);
    HttpGatewayResponse::Http(HttpResponse {
        code: StatusCode::BAD_REQUEST.as_u16(),
        headers: vec![(CONTENT_TYPE.to_string(), vec![TEXT_HTML.to_string()])],
        body: Bstr::from(format!(
            "<html><body><h1>{}</h1></body></html>",
            messages::BAD_REQUEST
        )),
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

/// Create a JSON HTTP response with the given status code and body.
fn create_json_response(
    status: StatusCode,
    body: String,
) -> HttpGatewayResponse {
    HttpGatewayResponse::Http(HttpResponse {
        code: status.as_u16(),
        headers: vec![(CONTENT_TYPE.to_string(), vec![APPLICATION_JSON.to_string()])],
        body: Bstr::from(body),
    })
}

/// Convert successful stake info response to HTTP response
fn convert_to_http_response(stake_info: Responses) -> HttpGatewayResponse {
    match stake_info {
        Responses::Ok(full_stake_info) => {
            let json_body = serde_json::to_string(&full_stake_info).unwrap_or_else(|_| {
                format!("{{\"error\":\"{}\"}}", messages::SERIALIZATION_FAILED)
            });
            create_json_response(StatusCode::OK, json_body)
        },
        Responses::NotFound => {
            let error_body = format!("{{\"error\":\"{}\"}}", messages::STAKE_ADDRESS_NOT_FOUND);
            create_json_response(StatusCode::NOT_FOUND, error_body)
        },
    }
}

/// Convert error response to HTTP response
fn convert_error_to_http_response(error: &ErrorResponses) -> HttpGatewayResponse {
    match error {
        ErrorResponses::NotFound => {
            let error_body = format!("{{\"error\":\"{}\"}}", messages::NOT_FOUND);
            create_json_response(StatusCode::NOT_FOUND, error_body)
        },
        ErrorResponses::ServerError(_) => {
            let error_body = format!("{{\"error\":\"{}\"}}", messages::INTERNAL_SERVER_ERROR);
            create_json_response(StatusCode::INTERNAL_SERVER_ERROR, error_body)
        },
        ErrorResponses::ServiceUnavailable(..) => {
            let error_body = format!("{{\"error\":\"{}\"}}", messages::SERVICE_UNAVAILABLE);
            create_json_response(StatusCode::SERVICE_UNAVAILABLE, error_body)
        },
        _ => {
            let error_body = format!("{{\"error\":\"{}\"}}", messages::UNKNOWN_ERROR);
            create_json_response(StatusCode::INTERNAL_SERVER_ERROR, error_body)
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
        body: Vec<u8>,
        headers: Headers,
        path: String,
        method: String,
    ) -> Option<HttpGatewayResponse> {
        const FUNCTION_NAME: &str = "reply";
        log::init(log::LevelFilter::Trace);

        let validation_result = validate_stake_route(&path);
        let response = match validation_result {
            Ok(stake_address) => {
                log::info!("Processing STAKE_ROUTE: {method} {path} {body:?} {headers:?}",);

                // For GET requests, use default values if body is empty
                let request: GetStakedAdaRequest = if body.is_empty() {
                    GetStakedAdaRequest::default()
                } else {
                    match serde_json::from_slice(&body) {
                        Ok(req) => req,
                        Err(err) => {
                            log::error!("request parse failed: {err}");
                            return Some(create_not_found_response(&method, &path));
                        },
                    }
                };

                let response = staked_ada_get(
                    &stake_address,
                    request.network,
                    request.asat,
                    shared::utils::common::auth::none_or_rbac::NoneOrRBAC::None(NoAuthorization),
                );

                match response {
                    WithErrorResponses::With(stake_info) => {
                        log::info!("processed STAKE_ROUTE successfully");
                        convert_to_http_response(stake_info)
                    },
                    WithErrorResponses::Error(error_response) => {
                        log::info!("processed STAKE_ROUTE  with error");
                        convert_error_to_http_response(&error_response)
                    },
                }
            },
            Err(StakedAdaError::InvalidPath { .. }) => create_not_found_response(&method, &path),
            _ => create_bad_request_response(&method, &path),
        };

        log::info!(
            "Request completed: {method} {path} -> {}",
            format_response_type(&response)
        );

        Some(response)
    }
}

/// Validates staked ada route and extracts stake address from it.
fn validate_stake_route(path: &str) -> Result<Cip19StakeAddress> {
    let route_regex = stake_route_regex();
    if let Some(captures) = route_regex.captures(&path.to_lowercase()) {
        if let Some(stake_address_match) = captures.get(1) {
            let stake_address = stake_address_match.as_str();
            let stake_address = Cip19StakeAddress::try_from(stake_address).map_err(|_| {
                StakedAdaError::InvalidStakeAddress {
                    address: stake_address.to_string(),
                }
            })?;
            return Ok(stake_address);
        }
        return Err(StakedAdaError::Validation {
            message: "Stake address is missing or has invalid format".to_string(),
        });
    }
    Err(StakedAdaError::InvalidPath {
        path: path.to_string(),
    })
}
