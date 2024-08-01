//! Public key type for RBAC metadata

use minicbor::{data::Tag, decode, Decode, Decoder};

/// Enum of possible public key type.
#[derive(Debug, PartialEq)]
pub(crate) enum SimplePublicKeyType {
    /// Undefined indicates skipped element.
    Undefined,
    /// Deleted indicates the key is deleted.
    Deleted, // Tag 31
    /// Ed25519 key
    Ed25519([u8; 32]), // Tag 32773
}

/// Enum of possible public key tag.
enum PublicKeyTag {
    /// Deleted Key tag 31.
    Deleted,
    /// Ed25519 Key tag 32773.
    Ed25519,
}

impl PublicKeyTag {
    /// Get the tag value.
    fn tag(self) -> Tag {
        match self {
            PublicKeyTag::Deleted => Tag::new(0x31),
            PublicKeyTag::Ed25519 => Tag::new(0x8005),
        }
    }
}

impl Decode<'_, ()> for SimplePublickeyType {
    fn decode(d: &mut Decoder, _ctx: &mut ()) -> Result<Self, decode::Error> {
        match d.datatype()? {
            minicbor::data::Type::Tag => {
                let tag = d.tag()?;
                match tag {
                    t if t == PublicKeyTag::Deleted.tag() => Ok(SimplePublickeyType::Deleted),
                    t if t == PublicKeyTag::Ed25519.tag() => {
                        let bytes = d.bytes()?;
                        let mut ed25519 = [0u8; 32];
                        if bytes.len() == 32 {
                            ed25519.copy_from_slice(&bytes);
                            Ok(SimplePublicKeyType::Ed25519(ed25519))
                        } else {
                            Err(decode::Error::message("Invalid length for Ed25519 key"))
                        }
                    },
                    _ => {
                        Err(decode::Error::message(
                            "Unknown tag for SimplePublicKeyType",
                        ))
                    },
                }
            },
            minicbor::data::Type::Undefined => Ok(SimplePublicKeyType::Undefined),
            _ => {
                Err(decode::Error::message(
                    "Invalid datatype for SimplePublicKeyType",
                ))
            },
        }
    }
}
