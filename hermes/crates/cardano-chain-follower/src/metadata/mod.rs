mod x509;

use minicbor::{decode, Decode, Decoder};
use strum::EnumDiscriminants;
use x509::X509Metadatum;

// FIXME:  Revisit all error handling 

#[derive(Debug, PartialEq, EnumDiscriminants)]
#[strum_discriminants(name(MetadataListType))]
enum MetadataList {
    X509Metadatum(X509Metadatum),
}

impl MetadataListType {
    fn from_u16(value: u16) -> Option<Self> {
        match value {
            509 => Some(MetadataListType::X509Metadatum),
            _ => None,
        }
    }
}

#[derive(Debug, PartialEq)]
struct Metadata {
    label: u16,
    tx_metadata: MetadataList,
}

impl Metadata {
    fn new(label: u16, tx_metadata: MetadataList) -> Self {
        Self { label, tx_metadata }
    }
}

impl Decode<'_, ()> for Metadata {
    fn decode(d: &mut Decoder, _ctx: &mut ()) -> Result<Self, decode::Error> {
        d.map()?;
        let label = d.u16()?;
        let metadata =
            MetadataListType::from_u16(label).ok_or(decode::Error::message("Invalid label"))?;
        match metadata {
            MetadataListType::X509Metadatum => {
                let x509_metadatum = X509Metadatum::decode(d, &mut ())?;
                Ok(Metadata::new(
                    label,
                    MetadataList::X509Metadatum(x509_metadatum),
                ))
            },
        }
    }
}
