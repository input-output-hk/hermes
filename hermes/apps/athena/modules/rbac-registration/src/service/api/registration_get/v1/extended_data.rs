//! A role extended data.

use std::collections::HashMap;

use serde::Serialize;

/// A role extended data.
#[derive(Debug, Eq, PartialEq, Clone, Serialize)]
pub(crate) struct ExtendedData(HashMap<u8, Vec<u8>>);

impl From<HashMap<u8, Vec<u8>>> for ExtendedData {
    fn from(value: HashMap<u8, Vec<u8>>) -> Self {
        Self(value)
    }
}

impl ExtendedData {
    pub(crate) fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}
