//! Implement API endpoint interfacing `DateTime`.

use core::fmt;

use derive_more::{From, Into};

/// Newtype for `DateTime<Utc>`. Should be used for API interfacing `DateTime<Utc>` only.
#[derive(Debug, Clone, From, Into)]
pub(crate) struct DateTime(chrono::DateTime<chrono::offset::Utc>);

impl fmt::Display for DateTime {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        self.0.to_rfc3339().fmt(f)
    }
}
