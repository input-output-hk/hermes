use c509_certificate::tbs_cert::TbsCert;
use minicbor::{decode, Decode, Decoder};

// ------------------x509------------------------

#[derive(Debug, PartialEq)]
pub(crate) struct X509DerCert(Vec<u8>);

impl Decode<'_, ()> for X509DerCert {
    fn decode(d: &mut Decoder, _ctx: &mut ()) -> Result<Self, decode::Error> {
        Ok(Self(d.bytes()?.to_vec()))
    }
}

// ------------------c509-----------------------

#[derive(Debug, PartialEq)]
pub(crate) enum C509Cert {
    C509CertInMetadatumReference(C509CertInMetadatumReference),
    C509Certificate(TbsCert),
}

impl Decode<'_, ()> for C509Cert {
    fn decode(d: &mut Decoder, ctx: &mut ()) -> Result<Self, decode::Error> {
        let arr_len = d
            .array()?
            .ok_or(decode::Error::message("c509 cert has indefinite length"))?;
        // C509CertInMetadatumReference must have 3 items
        if arr_len == 3 {
            Ok(Self::C509CertInMetadatumReference(
                C509CertInMetadatumReference::decode(d, ctx)?,
            ))
        } else {
            Ok(Self::C509Certificate(TbsCert::decode(d, ctx)?))
        }
    }
}

#[derive(Debug, PartialEq)]
pub(crate) struct C509CertInMetadatumReference {
    txn_output_field: u8,
    txn_output_index: u64,
    cert_ref: Option<Vec<u64>>,
}

impl Decode<'_, ()> for C509CertInMetadatumReference {
    fn decode(d: &mut Decoder, _ctx: &mut ()) -> Result<Self, decode::Error> {
        let txn_output_field = d.u8()?;
        let txn_output_index = d.u64()?;
        let cert_ref = match d.datatype()? {
            minicbor::data::Type::Array => {
                let len = d
                    .array()?
                    .ok_or(decode::Error::message("cert ref has indefinite length"))?;
                let mut arr = vec![];
                for _ in 0..len {
                    arr.push(d.u64()?);
                }
                Ok(Some(arr))
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
