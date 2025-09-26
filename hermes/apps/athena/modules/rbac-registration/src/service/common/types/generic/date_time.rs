//! Implement API endpoint interfacing `DateTime`.

use core::fmt;

use serde::{Serialize, Serializer};

/// Newtype for `DateTime<Utc>`. Should be used for API interfacing `DateTime<Utc>` only.
#[derive(Debug, Clone)]
pub(crate) struct DateTime(chrono::DateTime<chrono::offset::Utc>);

impl fmt::Display for DateTime {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        self.0.to_rfc3339().fmt(f)
    }
}

impl Serialize for DateTime {
    fn serialize<S>(
        &self,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl From<chrono::DateTime<chrono::offset::Utc>> for DateTime {
    fn from(dt: chrono::DateTime<chrono::offset::Utc>) -> Self {
        Self(dt)
    }
}
