//! `UUIDv4` Type.

use std::sync::LazyLock;

use regex::Regex;
use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize)]
pub(crate) struct UUIDv4(String);

pub(crate) const PATTERN: &str =
    "^[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-4[0-9a-fA-F]{3}-[89abAB][0-9a-fA-F]{3}-[0-9a-fA-F]{12}$";

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
impl TryFrom<&str> for UUIDv4 {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        value.to_string().try_into()
    }
}

impl TryFrom<String> for UUIDv4 {
    type Error = anyhow::Error;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        if !is_valid(&value) {
            anyhow::bail!("Invalid UUIDv4")
        }
        Ok(Self(value))
    }
}
