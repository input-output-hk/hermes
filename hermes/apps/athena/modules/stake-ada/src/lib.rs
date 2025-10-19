#![allow(dead_code)]
//! Catalyst Gateway API

mod api;
mod common;
mod rbac;
mod settings;
mod utils;

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

use regex::Regex;
use shared::utils::log;

use crate::{
    api::cardano::staking::{staked_ada_get, GetStakedAdaRequest},
    common::{
        auth::none::NoAuthorization,
        responses::{ErrorResponses, WithErrorResponses},
        types::cardano::cip19_stake_address::Cip19StakeAddress,
    },
};

const STAKE_ROUTE: &str = r"^/api/gateway/v1/cardano/assets/(stake1[a-z0-9]{53})$";
static STAKE_ROUTE_REGEX: OnceLock<Regex> = OnceLock::new();

fn stake_route_regex() -> &'static Regex {
    STAKE_ROUTE_REGEX.get_or_init(|| Regex::new(STAKE_ROUTE).expect("Invalid Regex"))
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

/// Convert successful stake info response to HTTP response
fn convert_to_http_response(
    stake_info: crate::api::cardano::staking::Responses
) -> HttpGatewayResponse {
    match stake_info {
        crate::api::cardano::staking::Responses::Ok(full_stake_info) => {
            let json_body = serde_json::to_string(&full_stake_info)
                .unwrap_or_else(|_| "{\"error\":\"Serialization failed\"}".to_string());

            HttpGatewayResponse::Http(HttpResponse {
                code: 200,
                headers: vec![(
                    "content-type".to_string(),
                    vec!["application/json".to_string()],
                )],
                body: Bstr::from(json_body),
            })
        },
        crate::api::cardano::staking::Responses::NotFound => {
            HttpGatewayResponse::Http(HttpResponse {
                code: 404,
                headers: vec![(
                    "content-type".to_string(),
                    vec!["application/json".to_string()],
                )],
                body: Bstr::from("{\"error\":\"Stake address not found\"}"),
            })
        },
    }
}

/// Convert error response to HTTP response
fn convert_error_to_http_response(error: ErrorResponses) -> HttpGatewayResponse {
    match error {
        ErrorResponses::NotFound => HttpGatewayResponse::Http(HttpResponse {
            code: 404,
            headers: vec![(
                "content-type".to_string(),
                vec!["application/json".to_string()],
            )],
            body: Bstr::from("{\"error\":\"Not found\"}"),
        }),
        ErrorResponses::ServerError(_) => HttpGatewayResponse::Http(HttpResponse {
            code: 500,
            headers: vec![(
                "content-type".to_string(),
                vec!["application/json".to_string()],
            )],
            body: Bstr::from("{\"error\":\"Internal server error\"}"),
        }),
        ErrorResponses::ServiceUnavailable(_, _) => HttpGatewayResponse::Http(HttpResponse {
            code: 503,
            headers: vec![(
                "content-type".to_string(),
                vec!["application/json".to_string()],
            )],
            body: Bstr::from("{\"error\":\"Service unavailable\"}"),
        }),
        _ => HttpGatewayResponse::Http(HttpResponse {
            code: 500,
            headers: vec![(
                "content-type".to_string(),
                vec!["application/json".to_string()],
            )],
            body: Bstr::from("{\"error\":\"Unknown error\"}"),
        }),
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
                let request: GetStakedAdaRequest = serde_json::from_slice(&body)
                    .inspect_err(|err| {
                        log::error!("request parse failed: {err}");
                    })
                    .ok()?;

                let response = staked_ada_get(
                    stake_address,
                    request.network,
                    request.asat,
                    common::auth::none_or_rbac::NoneOrRBAC::None(NoAuthorization),
                );

                match response {
                    WithErrorResponses::With(stake_info) => {
                        log::info!("processed STAKE_ROUTE successfully");
                        convert_to_http_response(stake_info)
                    },
                    WithErrorResponses::Error(error_response) => {
                        log::info!("processed STAKE_ROUTE  with error");
                        convert_error_to_http_response(error_response)
                    },
                }
            },
            Err(err) => {
                log::error!("Path validation failed: {err}");
                create_not_found_response(&method, &path)
            },
        };

        log::info!(
            "Request completed: {method} {path} -> {}",
            format_response_type(&response)
        );

        Some(response)
    }
}

fn validate_stake_route(path: &str) -> anyhow::Result<Cip19StakeAddress> {
    let route_regex = stake_route_regex();
    if let Some(captures) = route_regex.captures(&path.to_lowercase()) {
        if let Some(stake_address_match) = captures.get(1) {
            let stake_address = stake_address_match.as_str();
            let stake_address = Cip19StakeAddress::try_from(stake_address)?;
            return Ok(stake_address);
        }
        anyhow::bail!("Stake address is missing or has invalid format");
    }
    anyhow::bail!("Invalid path provided: {path}");
}
