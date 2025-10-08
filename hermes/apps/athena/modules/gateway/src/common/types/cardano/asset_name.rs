//! Cardano Native Asset Name.

use crate::common::types::string_types::impl_string_types;

impl_string_types!(AssetName, "string", "cardano:asset_name", is_valid);

impl From<&Vec<u8>> for AssetName {
    fn from(value: &Vec<u8>) -> Self {
        match String::from_utf8(value.clone()) {
            Ok(name) => {
                // UTF8 - Yay
                // Escape any `\` so its consistent with escaped ascii below.
                let name = name.replace('\\', r"\\");
                Self(name)
            },
            Err(_) => Self(value.escape_ascii().to_string()),
        }
    }
}

impl From<String> for AssetName {
    fn from(value: String) -> Self {
        Self(value)
    }
}
