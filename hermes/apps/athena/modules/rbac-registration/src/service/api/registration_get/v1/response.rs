//! An `/rbac/registrations` V1 endpoint responses.

use crate::service::api::registration_get::v1::registration_chain::RbacRegistrationChain;

/// Response enum for RBAC registration
#[derive(Debug, Clone)]
pub enum ResponsesV1 {
    /// Success returns a list of RBAC registration chain.
    Ok(RbacRegistrationChain),
    /// No valid registration.
    NotFound,
    /// Response for unprocessable content.
    UnprocessableContent(String),
    /// Response for internal server error.
    InternalServerError(String),
    /// Response for service unavailable.
    ServiceUnavailable(String),
}

impl ResponsesV1 {
    /// Convert response to HTTP status code
    pub fn status_code(&self) -> u16 {
        match self {
            ResponsesV1::Ok(_) => 200,
            ResponsesV1::NotFound => 400,
            ResponsesV1::UnprocessableContent(_) => 422,
            ResponsesV1::InternalServerError(_) => 500,
            ResponsesV1::ServiceUnavailable(_) => 503,
        }
    }

    /// Serialize response body to JSON
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        match self {
            ResponsesV1::Ok(data) => serde_json::to_string(data),
            ResponsesV1::NotFound => Ok("Not Found".to_string()),
            ResponsesV1::UnprocessableContent(msg) => {
                serde_json::to_string(&serde_json::json!({"Unprocessable Content": msg}))
            },
            ResponsesV1::InternalServerError(msg) => {
                serde_json::to_string(&serde_json::json!({"Internal Server Error": msg}))
            },
            ResponsesV1::ServiceUnavailable(msg) => {
                serde_json::to_string(&serde_json::json!({"Service Unavailable": msg}))
            },
        }
    }
}
