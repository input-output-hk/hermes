//! Response for auth event

use serde::Serialize;
use thiserror::Error;

/// Authentication token error (401 Unauthorized).
#[derive(Debug, Error, Clone, Serialize)]
pub enum AuthTokenError {
    /// Registration chain cannot be built.
    #[error("Unable to build registration chain, err: {0}")]
    BuildRegChain(String),
    /// RBAC token cannot be parsed.
    #[error("Fail to parse RBAC token string, err: {0}")]
    ParseRbacToken(String),
    /// Registration chain cannot be found.
    #[error("Registration not found for the auth token.")]
    RegistrationNotFound,
    /// Latest signing key cannot be found.
    #[error("Unable to get the latest signing key.")]
    LatestSigningKey,
    /// Missing auth token.
    #[error("Missing auth token")]
    MissingToken,
}

/// Authorization, token does not have required access rights (403 Forbidden).
#[derive(Debug, Error, Clone, Serialize)]
#[error("Insufficient Permission for Catalyst RBAC Token: {0:?}")]
pub struct AuthTokenAccessViolation(pub Vec<String>);

/// Auth response enum
#[derive(Debug, Clone)]
pub enum AuthResponse {
    /// Auth successful (200 OK)
    Ok,
    /// Invalid or missing token (401 Unauthorized)
    Unauthorized(AuthTokenError),
    /// Valid token but insufficient permissions (403 Forbidden)
    Forbidden(AuthTokenAccessViolation),
    /// External service unavailable/dependency error (503 Service Unavailable)
    ServiceUnavailable(String),
    /// Internal server error
    InternalServerError(String),
}

impl AuthResponse {
    /// Convert response to HTTP status code
    pub fn status_code(&self) -> u16 {
        match self {
            AuthResponse::Ok => 200,
            AuthResponse::Unauthorized(_) => 401,
            AuthResponse::Forbidden(_) => 403,
            AuthResponse::ServiceUnavailable(_) => 503,
            AuthResponse::InternalServerError(_) => 500,
        }
    }

    /// Serialize response body to JSON
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        match self {
            AuthResponse::Ok => Ok("Authentication success".to_string()),
            AuthResponse::Unauthorized(msg) => {
                serde_json::to_string(&serde_json::json!({"Unauthorized": msg.to_string()}))
            },
            AuthResponse::Forbidden(msg) => {
                serde_json::to_string(&serde_json::json!({"Forbidden": msg.to_string()}))
            },
            AuthResponse::ServiceUnavailable(msg) => {
                serde_json::to_string(&serde_json::json!({"Service Unavailable": msg.to_string()}))
            },
            AuthResponse::InternalServerError(msg) => {
                serde_json::to_string(
                    &serde_json::json!({"Internal Server Error": msg.to_string()}),
                )
            },
        }
    }
}
