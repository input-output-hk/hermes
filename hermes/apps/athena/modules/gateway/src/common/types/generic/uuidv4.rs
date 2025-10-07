//! `UUIDv4` Type.
//!
//! String Encoded `UUIDv4`

use std::sync::LazyLock;

use anyhow::bail;
use regex::Regex;
use serde_json::Value;

use crate::common::types::string_types::impl_string_types;

/// Title.
const TITLE: &str = "UUIDv4";
/// Description.
const DESCRIPTION: &str = "128 Bit UUID Version 4 - Random";
/// Example.
const EXAMPLE: &str = "c9993e54-1ee1-41f7-ab99-3fdec865c744";
/// Length of the hex encoded string
pub(crate) const ENCODED_LENGTH: usize = EXAMPLE.len();
/// Validation Regex Pattern
pub(crate) const PATTERN: &str =
    "^[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-4[0-9a-fA-F]{3}-[89abAB][0-9a-fA-F]{3}-[0-9a-fA-F]{12}$";
/// Format
pub(crate) const FORMAT: &str = "uuid";

/// Validate `UUIDv4` This part is done separately from the `PATTERN`
fn is_valid(uuidv4: &str) -> bool {
    /// Regex to validate `UUIDv4`
    #[allow(clippy::unwrap_used)] // Safe because the Regex is constant.
    static RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(PATTERN).unwrap());

    if RE.is_match(uuidv4) {
        if let Ok(uuid) = uuid::Uuid::parse_str(uuidv4) {
            return uuid.get_version() == Some(uuid::Version::Random);
        }
    }
    false
}

impl_string_types!(UUIDv4, "string", FORMAT, is_valid);

impl TryInto<uuid::Uuid> for UUIDv4 {
    type Error = uuid::Error;

    fn try_into(self) -> Result<uuid::Uuid, Self::Error> {
        uuid::Uuid::parse_str(&self.0)
    }
}
