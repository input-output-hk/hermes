//! Auth Module

shared::bindings_generate!({
    world: "hermes:app/hermes",
    path: "../../../../../wasm/wasi/wit",
    inline: "
        package hermes:app;

        world hermes {
            include wasi:cli/imports@0.2.6;
            import hermes:logging/api;
            import hermes:http-gateway/api;

            export hermes:http-gateway/event-auth;
        }
    ",
    share: ["hermes:logging"],
});

mod api_keys;
mod database;
mod rbac;
mod response;
mod token;
mod utils;
mod validation;

use shared::{bindings::hermes::cardano, utils::log};

use crate::{
    hermes::http_gateway::api::{AuthRequest, Bstr, HttpResponse},
    response::{AuthResponse, AuthTokenError},
    validation::checker_api_catalyst_auth,
};

export!(AuthComponent);

struct AuthComponent;

impl AuthComponent {
    /// Create HTTP response from `AuthResponse`
    fn make_response(auth: &AuthResponse) -> HttpResponse {
        let headers = vec![("content-type".to_string(), vec![
            "application/json".to_string()
        ])];
        // Attempt to serialize, fallback to 500 if it fails
        let (code, body) =
            match auth.to_json() {
                Ok(body) => (auth.status_code(), body),
                Err(e) => (
                    AuthResponse::InternalServerError(e.to_string()).status_code(),
                    serde_json::json!({
                        "error": format!("Internal Server Error: Failed to serialize response: {e}")
                    })
                    .to_string(),
                ),
            };
        HttpResponse {
            code,
            headers,
            body: Bstr::from(body),
        }
    }

    /// Validate a token and return HTTP response
    fn validate_token(
        token: &str,
        headers: Vec<(String, Vec<String>)>,
        network: cardano::api::CardanoNetwork,
    ) -> HttpResponse {
        let result = checker_api_catalyst_auth(headers, token, network);
        Self::make_response(&result)
    }
}

impl exports::hermes::http_gateway::event_auth::Guest for AuthComponent {
    fn validate_auth(request: AuthRequest) -> Option<HttpResponse> {
        log::init(log::LevelFilter::Info);

        let network = cardano::api::CardanoNetwork::Preprod;
        let token = extract_header!(request.headers, "Authorization", "Bearer");

        match request.auth_level {
            hermes::http_gateway::api::AuthLevel::Required => {
                if let Some(t) = token {
                    Some(Self::validate_token(&t, request.headers, network))
                } else {
                    Some(Self::make_response(&AuthResponse::Unauthorized(
                        AuthTokenError::MissingToken,
                    )))
                }
            },
            // If the auth is present, validate it, if not skip it
            hermes::http_gateway::api::AuthLevel::Optional => {
                if let Some(t) = token {
                    Some(Self::validate_token(&t, request.headers, network))
                } else {
                    Some(Self::make_response(&AuthResponse::Ok))
                }
            },
            hermes::http_gateway::api::AuthLevel::None => {
                Some(Self::make_response(&AuthResponse::Ok))
            },
        }
    }
}
