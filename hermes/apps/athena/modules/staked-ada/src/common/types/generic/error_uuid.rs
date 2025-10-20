//! Implement API endpoint interfacing `ErrorUuid`.

use derive_more::{From, Into};
use utoipa::ToSchema;
use uuid::Uuid;

/// Error Unique ID
#[derive(Debug, Clone, From, Into, ToSchema)]
pub(crate) struct ErrorUuid(Uuid);
