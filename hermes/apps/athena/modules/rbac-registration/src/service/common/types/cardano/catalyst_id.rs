//! A Catalyst identifier.

use catalyst_types::catalyst_id::CatalystId as CatalystIdInner;
use std::fmt::Display;

use serde::Serialize;

/// A Catalyst identifier.
#[derive(Debug, Clone, Serialize)]
pub(crate) struct CatalystId(String);

impl From<String> for CatalystId {
    fn from(id: String) -> Self {
        Self(id)
    }
}

impl From<&str> for CatalystId {
    fn from(id: &str) -> Self {
        Self(id.to_string())
    }
}

impl Into<String> for CatalystId {
    fn into(self) -> String {
        self.0
    }
}

impl From<CatalystIdInner> for CatalystId {
    fn from(value: CatalystIdInner) -> Self {
        Self(value.as_short_id().to_string())
    }
}

impl Display for CatalystId {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
