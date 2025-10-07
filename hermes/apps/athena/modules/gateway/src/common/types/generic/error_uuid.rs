//! Implement API endpoint interfacing `ErrorUuid`.

use derive_more::{From, Into};
use uuid::Uuid;

/// Error Unique ID
#[derive(Debug, Clone, From, Into)]
pub(crate) struct ErrorUuid(Uuid);
