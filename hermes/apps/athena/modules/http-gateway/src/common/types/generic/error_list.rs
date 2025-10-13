//! Implement newtype of `ErrorList`

use super::error_msg::ErrorMessage;
use crate::common::types::array_types::impl_array_types;

// List of Errors
impl_array_types!(ErrorList, ErrorMessage);
