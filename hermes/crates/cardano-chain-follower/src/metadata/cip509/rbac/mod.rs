//! Role Based Access Control (RBAC) metadata for CIP509.
//! Doc Reference: <https://github.com/input-output-hk/catalyst-CIPs/tree/x509-role-registration-metadata/CIP-XXXX>
//! CDDL Reference: <https://github.com/input-output-hk/catalyst-CIPs/blob/x509-role-registration-metadata/CIP-XXXX/x509-roles.cddl>

pub mod certs;
pub mod pub_key;
pub mod role_data;

use std::collections::HashMap;

use certs::{C509Cert, X509DerCert};
use minicbor::{decode, Decode, Decoder};
use pub_key::SimplePublicKeyType;
use role_data::RoleData;
use strum::FromRepr;

use super::decode_helper::{
    decode_any, decode_array_len, decode_bytes, decode_map_len, decode_u16,
};

/// Struct of Cip509 RBAC metadata.
#[derive(Debug, PartialEq, Clone, Default)]
pub struct Cip509RbacMetadata {
    /// Optional list of x509 certificates.
    pub x509_certs: Option<Vec<X509DerCert>>,
    /// Optional list of c509 certificates.
    /// The value can be either the c509 certificate or c509 metadatum reference.
    pub c509_certs: Option<Vec<C509Cert>>,
    /// Optional list of Public keys.
    pub pub_keys: Option<Vec<SimplePublicKeyType>>,
    /// Optional list of revocation list.
    pub revocation_list: Option<Vec<[u8; 16]>>,
    /// Optional list of role data.
    pub role_set: Option<Vec<RoleData>>,
    /// Optional map of purpose key data.
    /// Empty map if no purpose key data is present.
    pub purpose_key_data: HashMap<u16, Vec<u8>>,
}

/// The first valid purpose key.
const FIRST_PURPOSE_KEY: u16 = 200;
/// The last valid purpose key.
const LAST_PURPOSE_KEY: u16 = 299;

/// Enum of CIP509 RBAC metadata with its associated unsigned integer value.
#[derive(FromRepr, Debug, PartialEq)]
#[repr(u16)]
pub enum Cip509RbacMetadataInt {
    /// x509 certificates.
    X509Certs = 10,
    /// c509 certificates.
    C509Certs = 20,
    /// Public keys.
    PubKeys = 30,
    /// Revocation list.
    RevocationList = 40,
    /// Role data set.
    RoleSet = 100,
}

impl Cip509RbacMetadata {
    /// Create a new instance of `Cip509RbacMetadata`.
    pub(crate) fn new() -> Self {
        Self {
            x509_certs: None,
            c509_certs: None,
            pub_keys: None,
            revocation_list: None,
            role_set: None,
            purpose_key_data: HashMap::new(),
        }
    }

    /// Set the x509 certificates.
    fn set_x509_certs(&mut self, x509_certs: Vec<X509DerCert>) {
        self.x509_certs = Some(x509_certs);
    }

    /// Set the c509 certificates.
    fn set_c509_certs(&mut self, c509_certs: Vec<C509Cert>) {
        self.c509_certs = Some(c509_certs);
    }

    /// Set the public keys.
    fn set_pub_keys(&mut self, pub_keys: Vec<SimplePublicKeyType>) {
        self.pub_keys = Some(pub_keys);
    }

    /// Set the revocation list.
    fn set_revocation_list(&mut self, revocation_list: Vec<[u8; 16]>) {
        self.revocation_list = Some(revocation_list);
    }

    /// Set the role data set.
    fn set_role_set(&mut self, role_set: Vec<RoleData>) {
        self.role_set = Some(role_set);
    }
}

impl Decode<'_, ()> for Cip509RbacMetadata {
    fn decode(d: &mut Decoder, _ctx: &mut ()) -> Result<Self, decode::Error> {
        let map_len = decode_map_len(d, "Cip509RbacMetadata")?;

        let mut x509_rbac_metadata = Cip509RbacMetadata::new();

        for _ in 0..map_len {
            let key = decode_u16(d, "key in Cip509RbacMetadata")?;
            if let Some(key) = Cip509RbacMetadataInt::from_repr(key) {
                match key {
                    Cip509RbacMetadataInt::X509Certs => {
                        let x509_certs = decode_array_rbac(d, "x509 certificate")?;
                        x509_rbac_metadata.set_x509_certs(x509_certs);
                    },
                    Cip509RbacMetadataInt::C509Certs => {
                        let c509_certs = decode_array_rbac(d, "c509 certificate")?;
                        x509_rbac_metadata.set_c509_certs(c509_certs);
                    },
                    Cip509RbacMetadataInt::PubKeys => {
                        let pub_keys = decode_array_rbac(d, "public keys")?;
                        x509_rbac_metadata.set_pub_keys(pub_keys);
                    },
                    Cip509RbacMetadataInt::RevocationList => {
                        let revocation_list = decode_revocation_list(d)?;
                        x509_rbac_metadata.set_revocation_list(revocation_list);
                    },
                    Cip509RbacMetadataInt::RoleSet => {
                        let role_set = decode_array_rbac(d, "role set")?;
                        x509_rbac_metadata.set_role_set(role_set);
                    },
                }
            } else {
                if !(FIRST_PURPOSE_KEY..=LAST_PURPOSE_KEY).contains(&key) {
                    return Err(decode::Error::message(format!("Invalid purpose key set, should be with the range {FIRST_PURPOSE_KEY} - {LAST_PURPOSE_KEY}")));
                }
                x509_rbac_metadata
                    .purpose_key_data
                    .insert(key, decode_any(d)?);
            }
        }
        Ok(x509_rbac_metadata)
    }
}

/// Decode an array of type T.
fn decode_array_rbac<'b, T>(d: &mut Decoder<'b>, from: &str) -> Result<Vec<T>, decode::Error>
where T: Decode<'b, ()> {
    let len = decode_array_len(d, &format!("{from} Cip509RbacMetadata"))?;
    let mut vec = Vec::with_capacity(usize::try_from(len).map_err(decode::Error::message)?);
    for _ in 0..len {
        vec.push(T::decode(d, &mut ())?);
    }
    Ok(vec)
}

/// Decode an array of revocation list.
fn decode_revocation_list(d: &mut Decoder) -> Result<Vec<[u8; 16]>, decode::Error> {
    let len = decode_array_len(d, "revocation list Cip509RbacMetadata")?;
    let mut revocation_list =
        Vec::with_capacity(usize::try_from(len).map_err(decode::Error::message)?);
    for _ in 0..len {
        let arr: [u8; 16] = decode_bytes(d, "revocation list Cip509RbacMetadata")?
            .try_into()
            .map_err(|_| decode::Error::message("Invalid revocation list size"))?;
        revocation_list.push(arr);
    }
    Ok(revocation_list)
}
