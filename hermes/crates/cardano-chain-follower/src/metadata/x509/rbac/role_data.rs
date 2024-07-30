use minicbor::{decode, Decode, Decoder};

use super::X509RbacMetadataInt;

#[derive(Debug, PartialEq)]
pub(crate) struct RoleData {
    role_number: u8,
    role_signing_key: Option<KeyReference>,
    role_encryption_key: Option<KeyReference>,
    payment_key: Option<u64>,
    role_extended_data_keys: Option<RoleExtendedDataKeys>,
}

#[derive(Debug, PartialEq)]
struct RoleExtendedDataKeys {
    role_extended_data_keys: u64,
    role_extended_data_keys_value: Vec<u8>,
}

impl RoleData {
    fn new() -> Self {
        Self {
            role_number: 0,
            role_signing_key: None,
            role_encryption_key: None,
            payment_key: None,
            role_extended_data_keys: None,
        }
    }

    fn set_role_number(&mut self, role_number: u8) {
        self.role_number = role_number;
    }

    fn set_role_signing_key(&mut self, key: KeyReference) {
        self.role_signing_key = Some(key);
    }

    fn set_role_encryption_key(&mut self, key: KeyReference) {
        self.role_encryption_key = Some(key);
    }

    fn set_payment_key(&mut self, key: u64) {
        self.payment_key = Some(key);
    }
}

// #[derive(FromRepr, Debug, PartialEq)]
// #[repr(u8)]
// pub enum RoleDataInt {
//     RoleNumber = 0,
//     RoleSigningKey = 1,
//     RoleEncryptionKey = 2,
//     PaymentKey = 3,
// }

impl Decode<'_, ()> for RoleData {
    fn decode(d: &mut Decoder, ctx: &mut ()) -> Result<Self, decode::Error> {
        let map_len = d
            .map()?
            .ok_or(decode::Error::message("role set has indefinite length"))?;
        let mut role_data = RoleData::new();
        for _ in 0..map_len {
            match d.u64()? {
                0 => {
                    println!("role number");
                    let role_number = d.u8()?;
                    role_data.set_role_number(role_number);
                },
                1 => {
                    println!("role signing key");
                    role_data.set_role_signing_key(KeyReference::decode(d, ctx)?);
                },
                2 => {
                    println!("role encryption key");
                    role_data.set_role_encryption_key(KeyReference::decode(d, ctx)?);
                },
                3 => {
                    println!("payment key");
                    let payment_key = d.u64()?;
                    role_data.set_payment_key(payment_key);
                },
                extended_key => {
                    println!("role extended data keys");
                    let value = match d.datatype()? {
                        minicbor::data::Type::Bytes => d.bytes()?.to_vec(),
                        minicbor::data::Type::U8
                        | minicbor::data::Type::U16
                        | minicbor::data::Type::U32
                        | minicbor::data::Type::U64 => d.u64()?.to_be_bytes().to_vec(),
                        minicbor::data::Type::I8
                        | minicbor::data::Type::I16
                        | minicbor::data::Type::I32
                        | minicbor::data::Type::I64 => d.i64()?.to_be_bytes().to_vec(),
                        minicbor::data::Type::String => d.str()?.as_bytes().to_vec(),
                        _ => return Err(decode::Error::message("Data type not supported")),
                    };
                    role_data.role_extended_data_keys = Some(RoleExtendedDataKeys {
                        role_extended_data_keys: extended_key,
                        role_extended_data_keys_value: value,
                    });
                },
            }
        }
        Ok(role_data)
    }
}

#[derive(Debug, PartialEq)]
enum KeyReference {
    KeyLocalRef(KeyLocalRef),
    KeyHash(Vec<u8>),
}

impl Decode<'_, ()> for KeyReference {
    fn decode(d: &mut Decoder, ctx: &mut ()) -> Result<Self, decode::Error> {
        match d.datatype()? {
            minicbor::data::Type::Array => Ok(Self::KeyLocalRef(KeyLocalRef::decode(d, ctx)?)),
            minicbor::data::Type::Bytes => Ok(Self::KeyHash(d.bytes()?.to_vec())),
            _ => Err(decode::Error::message("Invalid data type")),
        }
    }
}

#[derive(Debug, PartialEq)]
struct KeyLocalRef {
    local_ref: LocalRef,
    key_offset: u64,
}

#[allow(dead_code)]
#[derive(Debug, PartialEq)]
pub enum LocalRef {
    X509Certs = X509RbacMetadataInt::X509Certs as isize, // 10
    C509Certs = X509RbacMetadataInt::C509Certs as isize, // 20
    SimplePublicKeys = X509RbacMetadataInt::SimplePublicKeys as isize, // 30
}

impl Decode<'_, ()> for KeyLocalRef {
    fn decode(d: &mut Decoder, _ctx: &mut ()) -> Result<Self, decode::Error> {
        d.array()?;
        let local_ref = match d.u8()? {
            10 => Ok(LocalRef::X509Certs),
            20 => Ok(LocalRef::C509Certs),
            30 => Ok(LocalRef::SimplePublicKeys),
            _ => Err(decode::Error::message("Invalid key local ref list")),
        }?;
        let key_offset = d.u64()?;
        Ok(Self {
            local_ref,
            key_offset,
        })
    }
}
