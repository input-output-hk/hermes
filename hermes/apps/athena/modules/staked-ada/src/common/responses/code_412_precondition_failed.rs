//! Define `Precondition Failed` response type.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::{common, common::types::array_types::impl_array_types};

/// The client has not sent valid data in its request, headers, parameters or body.
// #[derive(Object, Debug, Clone)]
#[derive(Debug, Clone, ToSchema)]
// #[oai(example)]
pub(crate) struct PreconditionFailed {
    /// Details of each error in the content that was detected.
    ///
    /// Note: This may not be ALL errors in the content, as validation of content can stop
    /// at any point an error is detected.
    detail: ContentErrorDetailList,
}

impl PreconditionFailed {
    /// Create a new `ContentErrorDetail` Response Payload.
    pub(crate) fn new(errors: Vec<anyhow::Error>) -> Self {
        let mut detail = vec![];
        for error in errors {
            detail.push(ContentErrorDetail::new(&error));
        }

        Self {
            detail: detail.into(),
        }
    }
}

// List of Content Error Details
impl_array_types!(ContentErrorDetailList, ContentErrorDetail);

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub(crate) struct ContentErrorDetail {
    /// The location of the error
    // #[oai(skip_serializing_if_is_none)]
    loc: Option<common::types::generic::error_list::ErrorList>,
    /// The error message.
    // #[oai(skip_serializing_if_is_none)]
    msg: Option<common::types::generic::error_msg::ErrorMessage>,
    /// The type of error
    // #[oai(rename = "type", skip_serializing_if_is_none)]
    err_type: Option<common::types::generic::error_msg::ErrorMessage>,
}

// impl Example for ContentErrorDetail {
//     /// Example for the `ContentErrorDetail` Payload.
//     fn example() -> Self {
//         Self {
//             loc: Some(vec!["body".into()].into()),
//             msg: Some("Value is not a valid dict.".into()),
//             err_type: Some("type_error.dict".into()),
//         }
//     }
// }

impl ContentErrorDetail {
    /// Create a new `ContentErrorDetail` Response Payload.
    pub(crate) fn new(error: &anyhow::Error) -> Self {
        // TODO: See if we can get more info from the error than this.
        Self {
            loc: None,
            msg: Some(error.to_string().into()),
            err_type: None,
        }
    }
}
