//! `UUIDv4` Type.
//!
//! String Encoded `UUIDv4`

use crate::common::types::string_types::impl_string_types;

impl_string_types!(UUIDv4, "string", FORMAT, is_valid);

impl TryInto<uuid::Uuid> for UUIDv4 {
    type Error = uuid::Error;

    fn try_into(self) -> Result<uuid::Uuid, Self::Error> {
        uuid::Uuid::parse_str(&self.0)
    }
}
