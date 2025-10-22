//! Define `Unauthorized` response type.

use utoipa::ToSchema;
use uuid::Uuid;

use crate::utils::common;

// Keep this message consistent with the response comment.
/// The client has not sent valid authentication credentials for the requested
/// resource.
#[derive(ToSchema)]
pub struct Unauthorized {
    /// Unique ID of this Server Error so that it can be located easily for debugging.
    id: common::types::generic::error_uuid::ErrorUuid,
    /// Error message.
    // Will not contain sensitive information, internal details or backtraces.
    msg: common::types::generic::error_msg::ErrorMessage,
}

impl Unauthorized {
    /// Create a new Payload.
    pub fn new(msg: Option<String>) -> Self {
        let msg = msg.unwrap_or(
            "Your request was not successful because it lacks valid authentication credentials for the requested resource.".to_string(),
        );
        let id = Uuid::new_v4();

        Self {
            id: id.into(),
            msg: msg.into(),
        }
    }
}
