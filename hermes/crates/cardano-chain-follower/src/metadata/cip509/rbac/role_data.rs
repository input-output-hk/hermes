//! Role data for RBAC metadata.

use std::collections::HashMap;

use minicbor::{decode, Decode, Decoder};
use strum::FromRepr;

use super::Cip509RbacMetadataInt;
use crate::metadata::cip509::decode_helper::{
    decode_any, decode_array_len, decode_bytes, decode_i64, decode_map_len, decode_u64, decode_u8,
};

/// Struct of role data.
#[derive(Debug, PartialEq, Clone, Default)]
pub struct RoleData {
    /// Role number.
    pub role_number: u8,
    /// Optional role signing key.
    pub role_signing_key: Option<KeyReference>,
    /// Optional role encryption key.
    pub role_encryption_key: Option<KeyReference>,
    /// Optional payment key.
    pub payment_key: Option<i64>,
    /// Optional role extended data keys.
    /// Empty map if no role extended data keys.
    pub role_extended_data_keys: HashMap<u8, Vec<u8>>,
}

/// The first valid role extended data key.
const FIRST_ROLE_EXT_KEY: u8 = 10;
/// The last valid role extended data key.
const LAST_ROLE_EXT_KEY: u8 = 99;

/// Enum of role data with its associated unsigned integer value.
#[allow(clippy::module_name_repetitions)]
#[derive(FromRepr, Debug, PartialEq)]
#[repr(u8)]
pub enum RoleDataInt {
    /// Role number.
    RoleNumber = 0,
    /// Role signing key.
    RoleSigningKey = 1,
    /// Role encryption key.
    RoleEncryptionKey = 2,
    /// Payment key.
    PaymentKey = 3,
}

impl Decode<'_, ()> for RoleData {
    fn decode(d: &mut Decoder, ctx: &mut ()) -> Result<Self, decode::Error> {
        let map_len = decode_map_len(d, "RoleData")?;
        let mut role_data = RoleData::default();
        for _ in 0..map_len {
            let key = decode_u8(d, "key in RoleData")?;
            if let Some(key) = RoleDataInt::from_repr(key) {
                match key {
                    RoleDataInt::RoleNumber => {
                        role_data.role_number = decode_u8(d, "RoleNumber in RoleData")?;
                    },
                    RoleDataInt::RoleSigningKey => {
                        role_data.role_signing_key = Some(KeyReference::decode(d, ctx)?);
                    },
                    RoleDataInt::RoleEncryptionKey => {
                        role_data.role_encryption_key = Some(KeyReference::decode(d, ctx)?);
                    },
                    RoleDataInt::PaymentKey => {
                        role_data.payment_key = Some(decode_i64(d, "PaymentKey in RoleData")?);
                    },
                }
            } else {
                if !(FIRST_ROLE_EXT_KEY..=LAST_ROLE_EXT_KEY).contains(&key) {
                    return Err(decode::Error::message(format!("Invalid role extended data key, should be with the range {FIRST_ROLE_EXT_KEY} - {LAST_ROLE_EXT_KEY}")));
                }
                role_data
                    .role_extended_data_keys
                    .insert(key, decode_any(d)?);
            }
        }
        Ok(role_data)
    }
}

/// Enum of key reference.
#[derive(Debug, PartialEq, Clone)]
pub enum KeyReference {
    /// Key local reference.
    KeyLocalRef(KeyLocalRef),
    /// Key hash.
    KeyHash(Vec<u8>),
}

impl Default for KeyReference {
    fn default() -> Self {
        KeyReference::KeyHash(Vec::new())
    }
}

impl Decode<'_, ()> for KeyReference {
    fn decode(d: &mut Decoder, ctx: &mut ()) -> Result<Self, decode::Error> {
        match d.datatype()? {
            minicbor::data::Type::Array => Ok(Self::KeyLocalRef(KeyLocalRef::decode(d, ctx)?)),
            minicbor::data::Type::Bytes => {
                Ok(Self::KeyHash(decode_bytes(d, "KeyHash in KeyReference")?))
            },
            _ => Err(decode::Error::message("Invalid data type for KeyReference")),
        }
    }
}

/// Struct of key local reference.
#[derive(Debug, PartialEq, Clone)]
pub struct KeyLocalRef {
    /// Local reference.
    local_ref: LocalRefInt,
    /// Key offset.
    key_offset: u64,
}

/// Enum of local reference with its associated unsigned integer value.
#[derive(FromRepr, Debug, PartialEq, Clone)]
#[repr(u8)]
enum LocalRefInt {
    /// x509 certificates.
    X509Certs = Cip509RbacMetadataInt::X509Certs as u8, // 10
    /// c509 certificates.
    C509Certs = Cip509RbacMetadataInt::C509Certs as u8, // 20
    /// Public keys.
    PubKeys = Cip509RbacMetadataInt::PubKeys as u8, // 30
}

impl Decode<'_, ()> for KeyLocalRef {
    fn decode(d: &mut Decoder, _ctx: &mut ()) -> Result<Self, decode::Error> {
        decode_array_len(d, "KeyLocalRef")?;
        let local_ref = LocalRefInt::from_repr(decode_u8(d, "LocalRef in KeyLocalRef")?)
            .ok_or(decode::Error::message("Invalid local reference"))?;
        let key_offset = decode_u64(d, "KeyOffset in KeyLocalRef")?;
        Ok(Self {
            local_ref,
            key_offset,
        })
    }
}
