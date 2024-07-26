// CDDL Reference: https://github.com/input-output-hk/catalyst-CIPs/blob/x509-role-registration-metadata/CIP-XXXX/x509-roles.cddl

use minicbor::{decode, Decode, Decoder};

#[derive(Debug, PartialEq)]
pub(crate) struct X509RbacMetadata {
    x509_certs: Option<Vec<X509DerCert>>, // [ + x509_der_cert ] bytes
    c509_certs: Option<Vec<C509Cert>>,
    pub_keys: Option<Vec<SimplePublickeyType>>,
    revocation_set: Option<Vec<Vec<[u8; 16]>>>,
    role_data_set: Option<Vec<RoleData>>,
}

#[allow(dead_code)]
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

    fn set_revocation_set(&mut self, revocation_set: Vec<Vec<[u8; 16]>>) {
        self.revocation_set = Some(revocation_set);
    }

    fn set_role_data_set(&mut self, role_data_set: Vec<RoleData>) {
        self.role_data_set = Some(role_data_set);
    }
}

impl Decode<'_, ()> for X509RbacMetadata {
    fn decode(d: &mut Decoder, _ctx: &mut ()) -> Result<Self, decode::Error> {
        // FIXME Assume that map already handle
        let map_len = d
            .map()?
            .ok_or(decode::Error::message("Data should be type Map"))?;
        let mut x509_rbac_metadata = X509RbacMetadata::new();
        for _ in 0..map_len {
            match d.u8()? {
                10 => {
                    println!("10");
                    let len = d.array()?.ok_or(decode::Error::message("TODO"))?;
                    println!("len: {}", len);
                    let mut x509_certs = vec![];
                    for _ in 0..len {
                        let cert = X509DerCert::decode(d, &mut ())?;
                        x509_certs.push(cert);
                    }
                    x509_rbac_metadata.set_x509_certs(x509_certs);
                },
                20 => {
                    println!("20");
                    let len = d.array()?.ok_or(decode::Error::message("TODO"))?;
                    let mut c509_cert = vec![];
                    for _ in 0..len {
                        let cert = C509Cert::decode(d, &mut ())?;
                        c509_cert.push(cert);
                    }
                    x509_rbac_metadata.set_c509_certs(c509_cert);
                },
                30 => {
                    println!("count 30");
                    let len = d.array()?.ok_or(decode::Error::message("TODO"))?;
                    let mut pub_keys = vec![];
                    for _ in 0..len {
                        let key = SimplePublickeyType::decode(d, &mut ())?;
                        pub_keys.push(key);
                    }
                    x509_rbac_metadata.set_pub_keys(pub_keys);
                },
                40 => {
                    todo!();
                    // let len = d.array()?.ok_or(decode::Error::message("TODO"))?;
                    // let mut revocation_set = vec![];
                    // for _ in 0..len {
                    //     let revocation = d.array()?.ok_or(decode::Error::message("TODO"))?;
                    //     let mut revocation_list = vec![];
                    //     for _ in 0..revocation {
                    //         revocation_list.push(d.bytes()?.to_vec());
                    //     }
                    //     revocation_set.push(revocation_list);
                    // }
                    // x509_rbac_metadata.set_revocation_set(
                    //     revocation_set
                    //         .try_into()
                    //         .map_err(|_| decode::Error::message("Invalid data size"))?,
                    // );
                },
                100 => {
                    let len = d.array()?.ok_or(decode::Error::message("TODO"))?;
                    let mut role_data_set = vec![];
                    for _ in 0..len {
                        let role_data = RoleData::decode(d, &mut ())?;
                        role_data_set.push(role_data);
                    }
                    x509_rbac_metadata.set_role_data_set(role_data_set);
                },
                _ => {
                    todo!();
                },
            }
        }
        Ok(x509_rbac_metadata)
    }
}

// ------------------------

#[derive(Debug, PartialEq)]
struct X509DerCert(Vec<u8>);

impl X509DerCert {
    fn new(cert: Vec<u8>) -> Self {
        Self(cert)
    }
}

impl Decode<'_, ()> for X509DerCert {
    fn decode(d: &mut Decoder, _ctx: &mut ()) -> Result<Self, decode::Error> {
        Ok(Self::new(d.bytes()?.to_vec()))
    }
}

// ------------------------

#[derive(Debug, PartialEq)]
struct C509Cert {
    c509_cert_in_metadatum_reference: C509CertInMetadatumReference,
    c509_certificate: Vec<C509>,
}

// FIXME - Should use C509 crate
#[derive(Debug, PartialEq)]
struct C509 {
    tbs_cert: Vec<u8>,
    issuer_sig_alg: Vec<u8>,
}

impl Decode<'_, ()> for C509 {
    fn decode(d: &mut Decoder, _ctx: &mut ()) -> Result<Self, decode::Error> {
        let tbs_cert = d.bytes()?.to_vec();
        let issuer_sig_alg = d.bytes()?.to_vec();
        Ok(Self {
            tbs_cert,
            issuer_sig_alg,
        })
    }
}
impl Decode<'_, ()> for C509Cert {
    fn decode(d: &mut Decoder, ctx: &mut ()) -> Result<Self, decode::Error> {
        let c509_cert_in_metadatum_reference = C509CertInMetadatumReference::decode(d, ctx)?;
        let mut cert = vec![];
        for _ in 0..d.array()?.ok_or(decode::Error::message("Invalid length"))? {
            cert.push(C509::decode(d, ctx)?);
        }
        Ok(Self {
            c509_cert_in_metadatum_reference,
            c509_certificate: cert,
        })
    }
}

#[derive(Debug, PartialEq)]
struct C509CertInMetadatumReference {
    txn_output_field: u8,
    txn_output_index: u64,
    cert_ref: Vec<u64>,
}

impl C509CertInMetadatumReference {
    fn new(field: u8, index: u64, cert_ref: Vec<u64>) -> Self {
        Self {
            txn_output_field: field,
            txn_output_index: index,
            cert_ref,
        }
    }
}

impl Decode<'_, ()> for C509CertInMetadatumReference {
    fn decode(d: &mut Decoder, _ctx: &mut ()) -> Result<Self, decode::Error> {
        if d.array()?.ok_or(decode::Error::message("Invalid length"))? != 3 {
            return Err(decode::Error::message("Invalid data size"));
        }
        let field = d.u8()?;
        let index = d.u64()?; // FIXME - Revisit
        let cert_ref = match d.datatype()? {
            minicbor::data::Type::Array => {
                let len = d
                    .array()?
                    .ok_or(decode::Error::message("Array should not be empty"))?;
                let mut arr = vec![];
                for _ in 0..len {
                    arr.push(d.u64()?);
                }
                Ok(arr)
            },
            // FIXME - Revisit
            _ => Ok(vec![d.u64()?]),
        }?;
        Ok(Self::new(field, index, cert_ref))
    }
}

// ------------------------
#[allow(dead_code)]
#[derive(Debug, PartialEq)]
enum SimplePublickeyType {
    Undefined,
    Deleted,
    Ed25519,
}

impl Decode<'_, ()> for SimplePublickeyType {
    fn decode(_d: &mut Decoder, _ctx: &mut ()) -> Result<Self, decode::Error> {
        todo!()
    }
}

// ------------------------

#[derive(Debug, PartialEq)]
struct RoleData {
    role_number: u8,
    role_signing_key: Option<KeyReference>,
    role_encryption_key: Option<KeyReference>,
    payment_key: Option<u64>,
    // role_extended_data_keys: any,
}

impl RoleData {
    fn new() -> Self {
        Self {
            role_number: 0,
            role_signing_key: None,
            role_encryption_key: None,
            payment_key: None,
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

impl Decode<'_, ()> for RoleData {
    fn decode(d: &mut Decoder, ctx: &mut ()) -> Result<Self, decode::Error> {
        // Assume that map is handled
        let mut role_data = RoleData::new();
        match d.u8()? {
            0 => {
                let role_number = d.u8()?;
                role_data.set_role_number(role_number);
            },
            1 => {
                role_data.set_role_signing_key(KeyReference::decode(d, ctx)?);
            },
            2 => {
                role_data.set_role_encryption_key(KeyReference::decode(d, ctx)?);
            },
            3 => {
                let payment_key = d.u64()?;
                role_data.set_payment_key(payment_key);
            },
            _ => {
                todo!();
            },
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
    keys: KeyLocalRefList,
    key_offset: u64,
}

impl KeyLocalRef {
    fn new(keys: KeyLocalRefList, offset: u64) -> Self {
        Self {
            keys,
            key_offset: offset,
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, PartialEq)]
enum KeyLocalRefList {
    X509List(Vec<X509DerCert>),
    C509List(Vec<C509Cert>),
    PubKeyList(Vec<SimplePublickeyType>),
}

impl Decode<'_, ()> for KeyLocalRefList {
    fn decode(d: &mut Decoder, _ctx: &mut ()) -> Result<Self, decode::Error> {
        if d.datatype()? == minicbor::data::Type::Array {
            // how can i distinguish between each type?
            todo!()
        } else {
            Err(decode::Error::message("Invalid data type"))
        }
    }
}

impl Decode<'_, ()> for KeyLocalRef {
    fn decode(d: &mut Decoder, _ctx: &mut ()) -> Result<Self, decode::Error> {
        let keys = KeyLocalRefList::decode(d, &mut ())?;
        let key_offset = d.u64()?;
        Ok(Self::new(keys, key_offset))
    }
}

#[cfg(test)]
mod test_metadata {
    use super::*;

    #[test]
    fn test_rbac() {
        let map = "a5";
        let x509_certs = "0a";
        let x509_certs_arr = "81";
        // bytes(568)
        let x509_certs_data = "590238308202343082019da00302010202145afc371daf301793cf0b1835a118c2f90363d5d9300d06092a864886f70d01010b05003045310b30090603550406130241553113301106035504080c0a536f6d652d53746174653121301f060355040a0c18496e7465726e6574205769646769747320507479204c7464301e170d3234303731313038353733365a170d3235303731313038353733365a3045310b30090603550406130241553113301106035504080c0a536f6d652d53746174653121301f060355040a0c18496e7465726e6574205769646769747320507479204c746430819f300d06092a864886f70d010101050003818d0030818902818100cd28e20b157ca70c85433c1689b1d5890ec479bdd1ffdcc5647ae12be9badf4af20764cd24bd64130831a57506dfbbdd3e924c96b259c6ccedf24d6a25618f0819643c739f145b733c3c94333e5937b499ada9a4ffc127457c7cb557f2f5623dcadea1e06f09129db9584b0aee949244b3252b52afde5d385c65e563a6efb07f0203010001a321301f301d0603551d0e0416041492eb169818b833588321957a846077aa239cf3a0300d06092a864886f70d01010b0500038181002e5f73333ce667e4172b252416eaa1d2e9681f59943724b4f366a8b930443ca6b69b12dd9debee9c8a6307695ee1884da4b00136195d1d8223d1c253ff408edfc8ed03af1819244c35d3843855fb9af86e84fb7636fa3f4a0fc396f6fb6fd16d3bcebde68a8bd81be61e8ee7d77e9f7f9804e03ebc31b4581313c955a667658b";
        let c509_certs = "14";
        let c509_certs_arr = "82";
        let metadatum_ref = "83010002";
        // bytes(140)
        let c509_arr = "82";
        let tbs = "004301f50d6b52464320746573742043411a63b0cd001a6955b90047010123456789ab01582102b1216ab96e5b3b3340f5bdf02e693f16213a04525ed44450b1019c2dfd3838ab0100";
        let issuer_sig_alg = "58406fc903015259a38c0800a3d0b2969ca21977e8ed6ec344964d4e1c6b37c8fb541274c3bb81b2f53073c5f101a5ac2a92886583b6a2679b6e682d2a26945ed0b2";
        let pub_keys = "181e";
        let pub_keys_arr = "83";

        let simple_pub_key_undefined = "f7";
        let pub_key = "8b004301f50d6b52464320746573742043411a63b0cd001a6955b90047010123456789ab01582102b1216ab96e5b3b3340f5bdf02e693f16213a04525ed44450b1019c2dfd3838ab010058406fc903015259a38c0800a3d0b2969ca21977e8ed6ec344964d4e1c6b37c8fb541274c3bb81b2f53073c5f101a5ac2a92886583b6a2679b6e682d2a26945ed0b2";

        let test_data = "".to_string()
            + map
            + x509_certs
            + x509_certs_arr
            + x509_certs_data
            + c509_certs
            + c509_certs_arr
            + metadatum_ref
            + c509_arr
            + tbs
            + issuer_sig_alg
            + pub_keys
            + pub_keys_arr
            + simple_pub_key_undefined
            + pub_key
            + simple_pub_key_undefined;
        let data = hex::decode(test_data).unwrap();

        let mut decoder = Decoder::new(&data);
        let rbac = X509RbacMetadata::decode(&mut decoder, &mut ()).unwrap();
        println!("{:?}", rbac);
    }
}
