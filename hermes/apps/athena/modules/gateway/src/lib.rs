#![allow(dead_code)]
//! Catalyst Gateway API

mod api;
mod common;
mod rbac;
mod settings;
mod utilities;
mod utils;

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

use shared::bindings::hermes::logging::api::{log, Level};

use crate::{
    api::cardano::staking::{Api, GetStakedAdaRequest},
    common::{auth::none::NoAuthorization, types::cardano::cip19_stake_address::Cip19StakeAddress},
};

const STAKE_ROUTE: &str = "/api/gateway/v1/cardano/assets/:stake_address";

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
/// Logs an info message
fn log_err(message: &str) {
    log(
        Level::Error,
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
        body: Vec<u8>,
        headers: Headers,
        path: String,
        method: String,
    ) -> Option<HttpGatewayResponse> {
        log_info(&format!("Processing HTTP request: {} {}", method, path));

        let response = match path.to_lowercase().as_str() {
            STAKE_ROUTE => {
                log_info(&format!(
                    "Processing STAKE_ROUTE: {} {} {:?} {:?}",
                    method, path, body, headers
                ));
                let request: GetStakedAdaRequest = serde_json::from_slice(&body)
                    .inspect_err(|err| {
                        log_err(&format!("request parse failed: {err}",));
                    })
                    .unwrap();
                let response = Api.staked_ada_get(
                    Cip19StakeAddress::try_from("asd").unwrap(),
                    request.network,
                    request.asat,
                    common::auth::none_or_rbac::NoneOrRBAC::None(NoAuthorization),
                );
                match response {
                    common::responses::WithErrorResponses::With(_v) => {
                        log_info(&format!("processed STAKE_ROUTE"))
                    },
                    common::responses::WithErrorResponses::Error(_error_responses) => {
                        log_info(&format!("processed with err STAKE_ROUTE"))
                    },
                }

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
