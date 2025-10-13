//! A Catalyst identifier.

// cSpell:ignoreRegExp cardano/Fftx

use anyhow::Context;
use catalyst_types::catalyst_id::CatalystId as CatalystIdInner;

/// A Catalyst identifier.
#[derive(Debug, Clone, PartialEq, Hash)]
pub(crate) struct CatalystId(CatalystIdInner);

impl From<CatalystIdInner> for CatalystId {
    fn from(value: CatalystIdInner) -> Self {
        Self(value.as_short_id())
    }
}

impl From<CatalystId> for CatalystIdInner {
    fn from(value: CatalystId) -> Self {
        value.0
    }
}

impl TryFrom<&str> for CatalystId {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        value
            .parse()
            .context("Invalid Catalyst ID")
            .map(|id: CatalystIdInner| Self(id.as_short_id()))
    }
}
