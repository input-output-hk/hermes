//! `UUIDv7` Type.
//!
//! String Encoded `UUIDv7`

use crate::common::types::string_types::impl_string_types;

impl_string_types!(UUIDv7, "string", FORMAT, is_valid);

impl TryInto<uuid::Uuid> for UUIDv7 {
    type Error = uuid::Error;

    fn try_into(self) -> Result<uuid::Uuid, Self::Error> {
        uuid::Uuid::parse_str(&self.0)
    }
}
