//! RBAC Registration Module

shared::bindings_generate!({
    world: "hermes:app/hermes",
    path: "../../../../../wasm/wasi/wit",
    inline: "
        package hermes:app;

        world hermes {
            include wasi:cli/imports@0.2.6;
            import hermes:cardano/api;
            import hermes:logging/api;
            import hermes:init/api;
            import hermes:http-gateway/api;

            export hermes:init/event;
            export hermes:http-gateway/event;
        }
    ",
    share: ["hermes:logging"],
});

use shared::{
    bindings::hermes::cardano,
    utils::log::{self, log_error, log_info},
};

use crate::{
    hermes::http_gateway::api::{Bstr, Headers, HttpGatewayResponse, HttpResponse},
    service::api::registration_get::v1::endpoint::endpoint_v1,
};

export!(RbacRegistrationComponent);

mod database;
mod rbac;
mod service;

struct RbacRegistrationComponent;

impl exports::hermes::init::event::Guest for RbacRegistrationComponent {
    fn init() -> bool {
        log::init(log::LevelFilter::Info);
        const FUNCTION_NAME: &str = "init";

        // Create a network instance
        let network = cardano::api::CardanoNetwork::Preprod;

        let network_resource = match cardano::api::Network::new(network) {
            Ok(nr) => nr,
            Err(e) => {
                log_error(
                    file!(),
                    FUNCTION_NAME,
                    "cardano::api::Network::new",
                    &format!("Failed to create network resource {network:?}: {e}"),
                    None,
                );
                return false;
            },
        };

        log_info(
            file!(),
            FUNCTION_NAME,
            "",
            &format!("🚀 Syncing network {network:?}, resource: {network_resource:?}"),
            None,
        );

        true
    }
}

impl exports::hermes::http_gateway::event::Guest for RbacRegistrationComponent {
    fn reply(
        _body: Vec<u8>,
        _headers: Headers,
        path: String,
        _method: String,
    ) -> Option<HttpGatewayResponse> {
        log::init(log::LevelFilter::Info);

        let network = cardano::api::CardanoNetwork::Preprod;
        let lookup = parse_query_param(&path, "lookup");
        let result = endpoint_v1(lookup, network);
        let code = result.status_code();

        Some(HttpGatewayResponse::Http(HttpResponse {
            code,
            headers: vec![("content-type".to_string(), vec![
                "application/json".to_string()
            ])],
            body: Bstr::from(match result.to_json() {
                Ok(json) => json,
                Err(e) => format!("{{\"error\": \"Failed to serialize response: {}\"}}", e),
            }),
        }))
    }
}

/// Extract query parameter from path.
// Temporary way to get the query parameter.
fn parse_query_param(
    path: &str,
    param_name: &str,
) -> Option<String> {
    path.split('?').nth(1)?.split('&').find_map(|pair| {
        let mut parts = pair.split('=');
        if parts.next()? == param_name {
            parts.next().map(|v| v.to_string())
        } else {
            None
        }
    })
}
