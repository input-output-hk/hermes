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

#[cfg(test)]
mod test_metadata {
    use super::*;

    #[test]
    fn test_metadata() {
        let map_metadata = "a1";
        let label_509 = "1901fd";

        let map_x509_metadatum = "a5";

        let purpose = "00";
        let uuid_bytes = "50ca7a1457ef9f4c7f9c747f8c4a4cfa6c";

        let txn_inputs_hash = "01";
        // Bytes(28) = 581c
        let txn_inputs_hash_bytes = "581c3659c8b63a7cee30ac575f9c618a0e25834d80e47ba4a640de718275";

        let prv_tx_id = "02";
        // Bytes(32) = 5820
        let prv_tx_id_bytes =
            "58203659c8b63a7cee30ac575f9c618a0e25834d80e47ba4a640de718275fac7269b";

        let validation_sig = "1863";
        // Bytes(64) = 5840
        let validation_sig_bytes = "5840f106679367e27502e84f05bfe429d915e19aff73a87a42d0ce26762acd611d45825b25fbc94f5a227d496b966189303e89f9a8ed4a4639eb5d80d78a92bb5403";

        let chunk_map = "a1";
        let chunks_type = "0b";
        let chunks_bytes = "8c58401b3b0308667876f9bdbad1cdee66b91bad4f4027a75f4cb10796b41d2dc74403a09803b63437388cd220488679bb4e83d4b83bac869160258c043a7b1018c0305840abc71f55de698ca3fe3c0daadc8beefd4402afdec4e8c8a9a48acc8abed30140430661c6441a8c8a1c712690b3c0face44168c894e494bfd7332d9ee8844418c58405f06352dba4cd9353c28343c2951c02e295dc02a2908e3e3a05350c25489444c4d595551c5835a79442ee1dd33002e86e095108697c2380e9d957844c396354c58405b68a8c55e3a7bb594fe207df3e2ff3b8741190fa55faeddb7f844117456643388859238984c7e7ffdae4e9d55d39cfbd199b79fac2344fdab293983b412bb5958403d13b5b41a1475dc55175a274707feef8a1b67652dba7efa1aa07dea5eef83582ae6ce55374bea770d7546f3a2d20ed3f7bcd5bc829f060ebd9fc9851100817e5840418c1fe325479c79e909ec04d6bad7ec2de88aa25bbe606346815fea9870c7e71e08cd3a3824e79ba8a8f52cf40987b4a808fbabde4b2fc3f8dd1b5445163e8758408cac62465a434bedcc773b5fbfeb280ba408f77958623d30035438bd7949c217f71cffeb55dd3ff11699463945ac942f17a839ff6efb1153f03b45e597ae05c358407ed3b7dfb117a335cf6d3e2b2bbfc1fd8cafeaf9f59ceedc16c2039d0de2821b0bcb49e7a1d0e07256dcada21c32045fe9221d4c0c8ff63f36d4e70a9c390b715840853baf42a600081b87968e0337417856306235c6535351efebe607b9705d76410d8283cf1523bb59d021fb574d6d1c406e7ab12711e0e0de5f4109f55f986bea5840e8e34c7df13666dfa8c9da862752f5c46f27e6a4fd757cee2b9678f8110c4e48d59504e72ff585b6c784c94a8935f85c9843f9703701cd08f18d734b03b201fd58402317a462e9128315d852791f190e69ec98f596bbdf9244254876bb1aa1ef9a28efb8878c8df23f9efd20684714e5b8a8ab1178abda2cd7e5de45721455c007214b0890a821d822025107558518635840f106679367e27502e84f05bfe429d915e19aff73a87a42d0ce26762acd611d45825b25fbc94f5a227d496b966189303e89f9a8ed4a4639eb5d80d78a92bb5403";
        let test_data = "".to_string()
            + map_metadata
            + label_509
            + map_x509_metadatum
            + purpose
            + uuid_bytes
            + txn_inputs_hash
            + txn_inputs_hash_bytes
            + prv_tx_id
            + prv_tx_id_bytes
            + chunk_map
            + chunks_type 
            + chunks_bytes
            + validation_sig
            + validation_sig_bytes;

        let data = hex::decode(test_data).unwrap();
        let mut decoder = Decoder::new(&data);
        let metadata = Metadata::decode(&mut decoder, &mut ()).unwrap();
        println!("{:?}", metadata);
    }
}
