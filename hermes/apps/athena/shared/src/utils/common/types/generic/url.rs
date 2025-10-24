//! Implementation of `Url` newtype.

use derive_more::{From, Into};

/// URL String
#[derive(Debug, Clone, From, Into)]
pub(crate) struct Url(url::Url);
