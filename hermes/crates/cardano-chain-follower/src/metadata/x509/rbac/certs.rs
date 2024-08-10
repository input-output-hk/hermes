//! Certificates for the RBAC metadata.

use c509_certificate::c509::C509;
use minicbor::{decode, Decode, Decoder};
use x509_cert::{der::Decode as x509Decode, Certificate};

// ------------------x509------------------------

/// A struct of X509 certificate.
#[derive(Debug, PartialEq)]
pub(crate) struct X509DerCert(Vec<u8>);

impl Decode<'_, ()> for X509DerCert {
    fn decode(d: &mut Decoder, _ctx: &mut ()) -> Result<Self, decode::Error> {
        let data = d.bytes()?;
        Certificate::from_der(data)
            .map_err(|_| decode::Error::message("Invalid x509 certificate"))?;
        Ok(Self(data.to_vec()))
    }
}

// ------------------c509-----------------------

/// Enum of possible c509 certificate.
#[derive(Debug, PartialEq)]
pub(crate) enum C509Cert {
    /// A c509 certificate in metadatum reference.
    C509CertInMetadatumReference(C509CertInMetadatumReference),
    /// A c509 certificate.
    C509Certificate(Box<C509>),
}

impl Decode<'_, ()> for C509Cert {
    fn decode(d: &mut Decoder, ctx: &mut ()) -> Result<Self, decode::Error> {
        if d.datatype()? == minicbor::data::Type::Array {
            let arr_len = d
                .array()?
                .ok_or(decode::Error::message("Error indefinite array in C509Cert"))?;
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
            let c509 = d.bytes()?;
            let mut c509_d = Decoder::new(c509);
            Ok(Self::C509Certificate(Box::new(C509::decode(
                &mut c509_d,
                ctx,
            )?)))
        }
    }
}

/// A struct of c509 certificate in metadatum reference.
#[derive(Debug, PartialEq)]
pub(crate) struct C509CertInMetadatumReference {
    /// Transaction output field.
    txn_output_field: u8,
    /// Transaction output index.
    txn_output_index: u64,
    /// Optional certificate reference.
    cert_ref: Option<Vec<u64>>,
}

impl Decode<'_, ()> for C509CertInMetadatumReference {
    fn decode(d: &mut Decoder, _ctx: &mut ()) -> Result<Self, decode::Error> {
        let txn_output_field = d.u8()?;
        let txn_output_index = d.u64()?;
        let cert_ref = match d.datatype()? {
            minicbor::data::Type::Array => {
                let len = d.array()?.ok_or(decode::Error::message(
                    "Error indefinite array in C509CertInMetadatumReference",
                ))?;
                let arr: Result<Vec<u64>, _> = (0..len).map(|_| d.u64()).collect();
                arr.map(Some)
            },
            minicbor::data::Type::Null => Ok(None),
            _ => Ok(Some(vec![d.u64()?])),
        }?;
        Ok(Self {
            txn_output_field,
            txn_output_index,
            cert_ref,
        })
    }
}
