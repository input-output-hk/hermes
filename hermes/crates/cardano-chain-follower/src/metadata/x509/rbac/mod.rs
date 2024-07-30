// CDDL Reference: https://github.com/input-output-hk/catalyst-CIPs/blob/x509-role-registration-metadata/CIP-XXXX/x509-roles.cddl

mod certs;
mod pub_key;
mod role_data;

use certs::{C509Cert, X509DerCert};
use minicbor::{decode, Decode, Decoder};
use pub_key::SimplePublickeyType;
use role_data::RoleData;
use strum::FromRepr;

#[derive(Debug, PartialEq)]
pub(crate) struct X509RbacMetadata {
    x509_certs: Option<Vec<X509DerCert>>,
    c509_certs: Option<Vec<C509Cert>>,
    pub_keys: Option<Vec<SimplePublickeyType>>,
    revocation_set: Option<Vec<[u8; 16]>>,
    role_data_set: Option<Vec<RoleData>>,
}

#[derive(FromRepr, Debug, PartialEq)]
#[repr(u8)]
pub enum X509RbacMetadataInt {
    X509Certs = 10,
    C509Certs = 20,
    SimplePublicKeys = 30,
    RevolcationList = 40,
    RoleSet = 100,
}

impl X509RbacMetadata {
    pub(crate) fn new() -> Self {
        Self {
            x509_certs: None,
            c509_certs: None,
            pub_keys: None,
            revocation_set: None,
            role_data_set: None,
        }
    }

    fn set_x509_certs(&mut self, x509_certs: Vec<X509DerCert>) {
        self.x509_certs = Some(x509_certs);
    }

    fn set_c509_certs(&mut self, c509_certs: Vec<C509Cert>) {
        self.c509_certs = Some(c509_certs);
    }

    fn set_pub_keys(&mut self, pub_keys: Vec<SimplePublickeyType>) {
        self.pub_keys = Some(pub_keys);
    }

    fn set_revocation_set(&mut self, revocation_set: Vec<[u8; 16]>) {
        self.revocation_set = Some(revocation_set);
    }

    fn set_role_data_set(&mut self, role_data_set: Vec<RoleData>) {
        self.role_data_set = Some(role_data_set);
    }
}

impl Decode<'_, ()> for X509RbacMetadata {
    fn decode(d: &mut Decoder, _ctx: &mut ()) -> Result<Self, decode::Error> {
        let map_len = d.map()?.ok_or(decode::Error::message(
            "x509 RBAC metadata has indefinite length",
        ))?;
        let mut x509_rbac_metadata = X509RbacMetadata::new();
        for _ in 0..map_len {
            match X509RbacMetadataInt::from_repr(d.u8()?).ok_or(decode::Error::message(
                "Invalid int representation of x509 RBAC Metadata",
            ))? {
                X509RbacMetadataInt::X509Certs => {
                    println!("10");
                    let x509_certs = decode_array(d)?;
                    x509_rbac_metadata.set_x509_certs(x509_certs);
                },
                X509RbacMetadataInt::C509Certs => {
                    println!("20");
                    let c509_certs = decode_array(d)?;
                    x509_rbac_metadata.set_c509_certs(c509_certs);
                },
                X509RbacMetadataInt::SimplePublicKeys => {
                    println!("30");
                    let pub_keys = decode_array(d)?;
                    x509_rbac_metadata.set_pub_keys(pub_keys);
                },
                X509RbacMetadataInt::RevolcationList => {
                    println!("40");
                    let revocation_set = decode_revocation_set(d)?;
                    x509_rbac_metadata.set_revocation_set(revocation_set);
                },
                X509RbacMetadataInt::RoleSet => {
                    println!("100");
                    let role_data_set = decode_array(d)?;
                    x509_rbac_metadata.set_role_data_set(role_data_set);
                },
            }
        }
        Ok(x509_rbac_metadata)
    }
}

fn decode_array<'b, T>(d: &mut Decoder<'b>) -> Result<Vec<T>, decode::Error>
where
    T: Decode<'b, ()>,
{
    let len = d
        .array()?
        .ok_or(decode::Error::message("Array has indefinite length"))?;
    let mut vec = Vec::with_capacity(len as usize);
    for _ in 0..len {
        vec.push(T::decode(d, &mut ())?);
    }
    Ok(vec)
}

fn decode_revocation_set(d: &mut Decoder) -> Result<Vec<[u8; 16]>, decode::Error> {
    let len = d.array()?.ok_or(decode::Error::message(
        "Revocation set array has indefinite length",
    ))?;
    let mut revocation_set = Vec::with_capacity(len as usize);
    for _ in 0..len {
        let arr: [u8; 16] = d
            .bytes()?
            .try_into()
            .map_err(|_| decode::Error::message("Invalid revocation list size"))?;
        revocation_set.push(arr);
    }
    Ok(revocation_set)
}

#[cfg(test)]
mod test_metadata {

    use super::*;

    #[test]
    fn test_rbac() {
        let map = "a5";
        // x509
        let x509_certs = "0a";
        let x509_certs_arr = "81";
        let x509_data = "590238308202343082019da00302010202145afc371daf301793cf0b1835a118c2f90363d5d9300d06092a864886f70d01010b05003045310b30090603550406130241553113301106035504080c0a536f6d652d53746174653121301f060355040a0c18496e7465726e6574205769646769747320507479204c7464301e170d3234303731313038353733365a170d3235303731313038353733365a3045310b30090603550406130241553113301106035504080c0a536f6d652d53746174653121301f060355040a0c18496e7465726e6574205769646769747320507479204c746430819f300d06092a864886f70d010101050003818d0030818902818100cd28e20b157ca70c85433c1689b1d5890ec479bdd1ffdcc5647ae12be9badf4af20764cd24bd64130831a57506dfbbdd3e924c96b259c6ccedf24d6a25618f0819643c739f145b733c3c94333e5937b499ada9a4ffc127457c7cb557f2f5623dcadea1e06f09129db9584b0aee949244b3252b52afde5d385c65e563a6efb07f0203010001a321301f301d0603551d0e0416041492eb169818b833588321957a846077aa239cf3a0300d06092a864886f70d01010b0500038181002e5f73333ce667e4172b252416eaa1d2e9681f59943724b4f366a8b930443ca6b69b12dd9debee9c8a6307695ee1884da4b00136195d1d8223d1c253ff408edfc8ed03af1819244c35d3843855fb9af86e84fb7636fa3f4a0fc396f6fb6fd16d3bcebde68a8bd81be61e8ee7d77e9f7f9804e03ebc31b4581313c955a667658b";
        // c509
        let c509_certs = "14";
        let c509_certs_arr = "81";
        let c509 = "8a004301f50d6b52464320746573742043411a63b0cd001a6955b90047010123456789ab01582102b1216ab96e5b3b3340f5bdf02e693f16213a04525ed44450b1019c2dfd3838ab0100"; //58406fc903015259a38c0800a3d0b2969ca21977e8ed6ec344964d4e1c6b37c8fb541274c3bb81b2f53073c5f101a5ac2a92886583b6a2679b6e682d2a26945ed0b2";
                                                                                                                                                                           // pub key
        let pub_keys = "181e";
        let pub_keys_arr = "81";
        let ed25519_tag = "d98005";
        // bytes(32) = 5820
        let pub_key = "58203b6a27bcceb6a42d62a3a8d02a6f0d73653215771de243a63ac048a18b59da29";
        let revocation_set = "1828";
        let revocation_set_arr = "82";

        let revocation_entry_1 = "50c13a67ee9608dc5966aaa91fe3b1f021";
        let revocation_entry_2 = "50431d7b744dcc4ac4359b7ee7ffa7be33";

        let role_set = "1864";
        let role_set_arr = "81";
        let role_data = "a5";
        let role_number = "00";
        let role_number_val = "01";
        let role_signing = "01";
        let role_signing_val = "820a00";
        let role_encryption = "02";
        let role_encryption_val = "50c13a67ee9608dc5966aaa91fe3b1f021";
        let payment_key = "03";
        let payment_key_val = "00";
        let role_extended_data = "0a";
        // Text("Test")
        let role_extended_data_val = "6454657374";

        let test_data = "".to_string()
            + map
            + x509_certs
            + x509_certs_arr
            + x509_data
            + c509_certs
            + c509_certs_arr
            + c509
            + pub_keys
            + pub_keys_arr
            + ed25519_tag
            + pub_key
            + revocation_set
            + revocation_set_arr
            + revocation_entry_1
            + revocation_entry_2
            + role_set
            + role_set_arr
            + role_data
            + role_number
            + role_number_val
            + role_signing
            + role_signing_val
            + role_encryption
            + role_encryption_val
            + payment_key
            + payment_key_val
            + role_extended_data
            + role_extended_data_val;
        let data = hex::decode(&test_data).unwrap();

        let mut decoder = Decoder::new(&data);
        let rbac = X509RbacMetadata::decode(&mut decoder, &mut ()).expect("Failed to decode");
        println!("{:?}", rbac);
    }
}
