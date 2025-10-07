//! `UUIDv7` Type.
//!
//! String Encoded `UUIDv7`

use regex::Regex;
use std::sync::LazyLock;

use crate::common::types::string_types::impl_string_types;

/// Title.
const TITLE: &str = "UUIDv7";
/// Description.
const DESCRIPTION: &str = "128 Bit UUID Version 7 - Timestamp + Random";
/// Example.
const EXAMPLE: &str = "01943a32-9f35-7a14-b364-36ad693465e6";
/// Length of the hex encoded string
pub(crate) const ENCODED_LENGTH: usize = EXAMPLE.len();
/// Validation Regex Pattern
pub(crate) const PATTERN: &str =
    "^[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-7[0-9a-fA-F]{3}-[89abAB][0-9a-fA-F]{3}-[0-9a-fA-F]{12}$";
/// Format
pub(crate) const FORMAT: &str = "uuidv7";

/// Validate `UUIDv7` This part is done separately from the `PATTERN`
fn is_valid(uuidv7: &str) -> bool {
    /// Regex to validate `UUIDv7`
    #[allow(clippy::unwrap_used)] // Safe because the Regex is constant.
    static RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(PATTERN).unwrap());

    if RE.is_match(uuidv7) {
        if let Ok(uuid) = uuid::Uuid::parse_str(uuidv7) {
            return uuid.get_version() == Some(uuid::Version::SortRand);
        }
    }
    false
}

impl_string_types!(UUIDv7, "string", FORMAT, is_valid);

impl TryInto<uuid::Uuid> for UUIDv7 {
    type Error = uuid::Error;

    fn try_into(self) -> Result<uuid::Uuid, Self::Error> {
        uuid::Uuid::parse_str(&self.0)
    }
}
