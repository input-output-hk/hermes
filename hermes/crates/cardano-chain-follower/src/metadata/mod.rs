//! Cardano metadata module

mod x509;

use minicbor::{decode, Decode, Decoder};
use strum::EnumDiscriminants;
use x509::X509Metadatum;

/// Enum of metadata currently supported
#[derive(Debug, PartialEq, EnumDiscriminants)]
#[strum_discriminants(name(MetadataListType))]
enum MetadataList {
    /// x509 metadatum
    X509Metadatum(X509Metadatum),
}

impl MetadataListType {
    /// Convert associated label unsigned integer to enum.
    fn from_u16(value: u16) -> Option<Self> {
        match value {
            509 => Some(MetadataListType::X509Metadatum),
            _ => None,
        }
    }
}

/// Struct of metadata
#[derive(Debug, PartialEq)]
struct Metadata {
    /// A label of the metadata
    label: u16,
    /// A possible list of metadata currently supported
    tx_metadata: MetadataList,
}

impl Decode<'_, ()> for Metadata {
    fn decode(d: &mut Decoder, _ctx: &mut ()) -> Result<Self, decode::Error> {
        // Map of label
        d.map()?;
        let label = d.u16()?;
        let metadata =
            MetadataListType::from_u16(label).ok_or(decode::Error::message("Invalid label"))?;
        match metadata {
            MetadataListType::X509Metadatum => {
                let x509_metadatum = X509Metadatum::decode(d, &mut ())?;
                Ok(Self {
                    label,
                    tx_metadata: MetadataList::X509Metadatum(x509_metadatum),
                })
            },
        }
    }
}

#[cfg(test)]
mod test_metadata {
    use super::*;

    #[test]
    fn test_raw_metadata() {
        let data = "a11901fda50050ca7a1457ef9f4c7f9c747f8c4a4cfa6c01508d1f34f63e927739c29bbdb2fe3263ff0258204d3f576f26db29139981a69443c2325daa812cc353a31b5a4db794a5bcbb06c20a8d5840a50a81590238308202343082019da00302010202145afc371daf301793cf0b1835a118c2f90363d5d9300d06092a864886f70d01010b05003045310b300906035840550406130241553113301106035504080c0a536f6d652d53746174653121301f060355040a0c18496e7465726e6574205769646769747320507479204c74643058401e170d3234303731313038353733365a170d3235303731313038353733365a3045310b30090603550406130241553113301106035504080c0a536f6d652d537458406174653121301f060355040a0c18496e7465726e6574205769646769747320507479204c746430819f300d06092a864886f70d010101050003818d00308189025840818100cd28e20b157ca70c85433c1689b1d5890ec479bdd1ffdcc5647ae12be9badf4af20764cd24bd64130831a57506dfbbdd3e924c96b259c6ccedf24d6a255840618f0819643c739f145b733c3c94333e5937b499ada9a4ffc127457c7cb557f2f5623dcadea1e06f09129db9584b0aee949244b3252b52afde5d385c65e563a65840efb07f0203010001a321301f301d0603551d0e0416041492eb169818b833588321957a846077aa239cf3a0300d06092a864886f70d01010b0500038181002e5f584073333ce667e4172b252416eaa1d2e9681f59943724b4f366a8b930443ca6b69b12dd9debee9c8a6307695ee1884da4b00136195d1d8223d1c253ff408edfc8ed584003af1819244c35d3843855fb9af86e84fb7636fa3f4a0fc396f6fb6fd16d3bcebde68a8bd81be61e8ee7d77e9f7f9804e03ebc31b4581313c955a667658b1481583f588b004301f50d6b52464320746573742043411a63b0cd001a6955b90047010123456789ab01582102b1216ab96e5b3b3340f5bdf02e693f16213a04525ed458404450b1019c2dfd3838ab010058406fc903015259a38c0800a3d0b2969ca21977e8ed6ec344964d4e1c6b37c8fb541274c3bb81b2f53073c5f101a5ac2a928865584083b6a2679b6e682d2a26945ed0b2181e81d9800558203b6a27bcceb6a42d62a3a8d02a6f0d73653215771de243a63ac048a18b59da2918288250667e69bd56a0583ffbd2d4db363e3bb017a150431d7b744dcc4ac4359b7ee7ffa7be33186481a5000001820a000250667e69bd56a0fbd2d4db363e3bb017a103000a64546573741863584004603e3ab833fc0063f0aad073bcce0a0dc97e475ba9418bad8746f90699c6043181563da067190ce97bc658125343195718298bf731b5f641c8356ca99f0f05";
        let hex_data = hex::decode(&data).expect("Failed to decode hex data");
        let mut decoder = Decoder::new(&hex_data);
        let _metadata = Metadata::decode(&mut decoder, &mut ()).expect("Failed to decode metadata");
    }
}
