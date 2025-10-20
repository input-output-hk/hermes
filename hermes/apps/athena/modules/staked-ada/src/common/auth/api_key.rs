//! API Key authorization scheme is used ONLY by internal endpoints.
//!
//! Its purpose is to prevent their use externally, if they were accidentally exposed.
//!
//! It is NOT to be used on any endpoint intended to be publicly facing.

use anyhow::{bail, Result};
use http::HeaderMap;

/// The header name that holds the API Key
pub(crate) const API_KEY_HEADER: &str = "X-API-Key";

/// Check if the API Key is correctly set.
/// Returns an error if it is not.
pub(crate) fn check_api_key(_headers: &HeaderMap) -> Result<()> {
    // if let Some(key) = headers.get(API_KEY_HEADER) {
    //     if Settings::check_internal_api_key(key.to_str()?) {
    //         return Ok(());
    //     }
    // }
    bail!("Invalid API Key");
}
