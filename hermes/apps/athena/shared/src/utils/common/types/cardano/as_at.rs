//! Query Parameter that can take either a Blockchain slot Number or Unix Epoch timestamp.
//!
//! Allows better specifying of times that restrict a GET endpoints response.

//! Hex encoded 28 byte hash.
//!
//! Hex encoded string which represents a 28 byte hash.

use std::fmt::{self, Display};

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::utils::common::types::cardano::slot_no::SlotNo;

/// As at time from query string parameter.
/// Store (Whence, When and decoded `SlotNo`) in a tuple for easier access.
#[derive(Debug, Eq, PartialEq, Hash, Serialize, Deserialize, ToSchema)]
pub struct AsAt((String, u64, SlotNo));

impl From<AsAt> for SlotNo {
    fn from(value: AsAt) -> Self {
        value.0 .2
    }
}

impl Display for AsAt {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        write!(f, "{}:{}", self.0 .0, self.0 .1)
    }
}
