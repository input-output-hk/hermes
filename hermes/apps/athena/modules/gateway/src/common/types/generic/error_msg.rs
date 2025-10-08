//! Generic Error Messages

// cspell: words impls

use crate::common::types::string_types::impl_string_types;

impl_string_types!(ErrorMessage, "string", "error", is_valid);

#[allow(clippy::derivable_impls)]
impl Default for ErrorMessage {
    fn default() -> Self {
        Self(String::default())
    }
}

impl From<String> for ErrorMessage {
    fn from(val: String) -> Self {
        Self(val)
    }
}

impl From<&str> for ErrorMessage {
    fn from(val: &str) -> Self {
        Self(val.to_owned())
    }
}
