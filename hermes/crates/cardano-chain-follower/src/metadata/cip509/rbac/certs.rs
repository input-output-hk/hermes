//! Certificates for the RBAC metadata.

use c509_certificate::c509::C509;
use minicbor::{decode, Decode, Decoder};
use x509_cert::{der::Decode as x509Decode, Certificate};

use crate::metadata::cip509::decode_helper::{
    decode_array_len, decode_bytes, decode_u64, decode_u8,
};

// ------------------x509------------------------

/// A struct of X509 certificate.
#[derive(Debug, PartialEq, Clone)]
pub struct X509DerCert(pub Vec<u8>);

impl Decode<'_, ()> for X509DerCert {
    fn decode(d: &mut Decoder, _ctx: &mut ()) -> Result<Self, decode::Error> {
        let data = decode_bytes(d, "X509DerCert")?;
        Certificate::from_der(&data)
            .map_err(|_| decode::Error::message("Invalid x509 certificate"))?;
        Ok(Self(data.clone()))
    }
}

// ------------------c509-----------------------

/// Enum of possible c509 certificate.
#[derive(Debug, PartialEq, Clone)]
pub enum C509Cert {
    /// A c509 certificate in metadatum reference.
    C509CertInMetadatumReference(C509CertInMetadatumReference),
    /// A c509 certificate.
    C509Certificate(Box<C509>),
}

impl Decode<'_, ()> for C509Cert {
    fn decode(d: &mut Decoder, ctx: &mut ()) -> Result<Self, decode::Error> {
        if d.datatype()? == minicbor::data::Type::Array {
            let arr_len = decode_array_len(d, "C509Cert")?;
            // C509CertInMetadatumReference must have 3 items
            if arr_len == 3 {
                Ok(Self::C509CertInMetadatumReference(
                    C509CertInMetadatumReference::decode(d, ctx)?,
                ))
            } else {
                Err(decode::Error::message(
                    "Invalid length C509CertInMetadatumReference, expected 3",
                ))
            }
        } else {
            // Consuming the c509 bytes
            let c509 = decode_bytes(d, "C509Cert")?;
            let mut c509_d = Decoder::new(&c509);
            Ok(Self::C509Certificate(Box::new(C509::decode(
                &mut c509_d,
                ctx,
            )?)))
        }
    }
}

/// A struct of c509 certificate in metadatum reference.
#[derive(Debug, PartialEq, Clone)]
pub struct C509CertInMetadatumReference {
    /// Transaction output field.
    txn_output_field: u8,
    /// Transaction output index.
    txn_output_index: u64,
    /// Optional certificate reference.
    cert_ref: Option<Vec<u64>>,
}

impl Decode<'_, ()> for C509CertInMetadatumReference {
    fn decode(d: &mut Decoder, _ctx: &mut ()) -> Result<Self, decode::Error> {
        let txn_output_field = decode_u8(d, "txn output field in C509CertInMetadatumReference")?;
        let txn_output_index = decode_u64(d, "txn output index in C509CertInMetadatumReference")?;
        let cert_ref = match d.datatype()? {
            minicbor::data::Type::Array => {
                let len = decode_array_len(d, "cert ref in C509CertInMetadatumReference")?;
                let arr: Result<Vec<u64>, _> = (0..len).map(|_| d.u64()).collect();
                arr.map(Some)
            },
            minicbor::data::Type::Null => Ok(None),
            _ => Ok(Some(vec![decode_u64(d, "C509CertInMetadatumReference")?])),
        }?;
        Ok(Self {
            txn_output_field,
            txn_output_index,
            cert_ref,
        })
    }
}
