//! An `/rbac/registrations` V1 endpoint responses.

use serde::{Deserialize, Serialize};

use crate::service::api::registration_get::v1::registration_chain::RbacRegistrationChain;

/// HTTP status codes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HttpStatus {
    Ok = 200,
    NotFound = 404,
    UnprocessableContent = 422,
    InternalServerError = 500,
    ServiceUnavailable = 503,
}

/// Simplified response enum without Poem dependencies
#[derive(Debug, Clone)]
pub enum ResponsesV1 {
    /// Success returns a list of registration transaction ids.
    Ok(RbacRegistrationChain),

    /// No valid registration.
    NotFound,
    /// Response for unprocessable content.
    UnprocessableContent(String),
    /// Internal server error
    InternalServerError(String),
    ServiceUnavailable(String),
}

impl ResponsesV1 {
    /// Convert response to HTTP status code
    pub fn status_code(&self) -> HttpStatus {
        match self {
            ResponsesV1::Ok(_) => HttpStatus::Ok,
            ResponsesV1::NotFound => HttpStatus::NotFound,
            ResponsesV1::UnprocessableContent(e) => HttpStatus::UnprocessableContent,
            ResponsesV1::InternalServerError(e) => HttpStatus::InternalServerError,
            ResponsesV1::ServiceUnavailable(e) => HttpStatus::ServiceUnavailable,
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

pub type AllResponsesV1 = ResponsesV1;
