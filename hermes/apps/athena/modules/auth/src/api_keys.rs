//! API Key auth scheme is used ONLY by internal endpoints.
//!
//! Its purpose is to prevent their use externally, if they were accidentally exposed.
//!
//! It is NOT to be used on any endpoint intended to be publicly facing.

use std::env;

use anyhow::{bail, Result};

use crate::{extract_header, hermes::http_gateway::api::Headers};

/// The header name that holds the API Key
pub(crate) const API_KEY_HEADER: &str = "X-API-Key";

/// Check if the API Key is correctly set.
pub(crate) fn check_api_key(headers: &Headers) -> Result<()> {
    if let Some(key) = extract_header!(headers, API_KEY_HEADER) {
        if check_internal_api_key(&key) {
            return Ok(());
        }
    }
    bail!("Invalid API Key");
}

/// Check a given key matches the internal API Key
fn check_internal_api_key(value: &str) -> bool {
    // TODO: This should be moved to application setting/config
    match env::var("INTERNAL_API_KEY") {
        Ok(expected_key) => value == expected_key,
        Err(_) => false,
    }
}
