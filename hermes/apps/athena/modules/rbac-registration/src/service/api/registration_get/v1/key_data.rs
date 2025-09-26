//! A role data key information.

use std::collections::HashMap;

use anyhow::Context;
use c509_certificate::c509::C509;
use cardano_blockchain_types::Point;
use ed25519_dalek::VerifyingKey;
use minicbor::{Encode, Encoder};
use rbac_registration::{
    cardano::cip509::{KeyLocalRef, LocalRefInt, PointData},
    registration::cardano::RegistrationChain,
};
use serde::Serialize;

use crate::service::{
    api::registration_get::{key_type::KeyType, v1::binary_data::HexEncodedBinaryData},
    common::types::generic::{
        date_time::DateTime as ServiceDateTime, ed25519_public_keys::Ed25519HexEncodedPublicKey,
    },
};
use chrono::{DateTime, Utc};
use x509_cert::{certificate::Certificate as X509Certificate, der::Encode as _};

/// A role data key information.
#[derive(Debug, Clone, Serialize)]
pub(crate) struct KeyData {
    /// Indicates if the data is persistent or volatile.
    is_persistent: bool,
    /// A time when the data was added.
    time: ServiceDateTime,
    /// A type of the key.
    key_type: KeyType,
    /// A value of the key.
    key_value: Option<HexEncodedBinaryData>,
}

impl KeyData {
    /// Creates a new `KeyData` instance.
    pub fn new(
        is_persistent: bool,
        time: DateTime<Utc>,
        key_ref: &KeyLocalRef,
        point: &Point,
        chain: &RegistrationChain,
    ) -> anyhow::Result<Self> {
        let key_value;

        let key_type = match key_ref.local_ref {
            LocalRefInt::X509Certs => {
                key_value = encode_x509(chain.x509_certs(), key_ref.key_offset, point)?;
                KeyType::X509
            },
            LocalRefInt::C509Certs => {
                key_value = encode_c509(chain.c509_certs(), key_ref.key_offset, point)?;
                KeyType::C509
            },
            LocalRefInt::PubKeys => {
                key_value = convert_pub_key(chain.simple_keys(), key_ref.key_offset, point)?;
                KeyType::Pubkey
            },
        };

        Ok(Self {
            is_persistent: is_persistent.into(),
            // FIXME
            time: time.into(),
            key_type,
            key_value,
        })
    }
}

/// Finds a X509 certificate with given offset and point and hex encodes it.
fn encode_x509(
    certs: &HashMap<usize, Vec<PointData<Option<X509Certificate>>>>,
    offset: usize,
    point: &Point,
) -> anyhow::Result<Option<HexEncodedBinaryData>> {
    certs
        .get(&offset)
        .with_context(|| format!("Invalid X509 certificate offset: {offset:?}"))?
        .iter()
        .find(|d| d.point() == point)
        .with_context(|| format!("Unable to find X509 certificate for the given point {point}"))?
        .data()
        .as_ref()
        .map(|cert| {
            cert.to_der()
                .context("Failed to encode X509 certificate")
                .map(Into::into)
        })
        .transpose()
}

/// Finds a C509 certificate with given offset and point and hex encodes it.
fn encode_c509(
    certs: &HashMap<usize, Vec<PointData<Option<C509>>>>,
    offset: usize,
    point: &Point,
) -> anyhow::Result<Option<HexEncodedBinaryData>> {
    certs
        .get(&offset)
        .with_context(|| format!("Invalid C509 certificate offset: {offset:?}"))?
        .iter()
        .find(|d| d.point() == point)
        .with_context(|| format!("Unable to find C509 certificate for the given point {point}"))?
        .data()
        .as_ref()
        .map(|cert| {
            let mut buffer = Vec::new();
            let mut e = Encoder::new(&mut buffer);
            cert.encode(&mut e, &mut ())
                .ok()
                .map(|()| buffer.into())
                .context("Failed to encode C509 certificate")
        })
        .transpose()
}

/// Finds a public key with the given offset and point and converts it.
fn convert_pub_key(
    keys: &HashMap<usize, Vec<PointData<Option<VerifyingKey>>>>,
    offset: usize,
    point: &Point,
) -> anyhow::Result<Option<HexEncodedBinaryData>> {
    Ok(keys
        .get(&offset)
        .with_context(|| format!("Invalid pub key offset: {offset}"))?
        .iter()
        .find(|d| d.point() == point)
        .with_context(|| format!("Unable to find pub key for the given point {point}"))?
        .data()
        .map(|k| Ed25519HexEncodedPublicKey::from(k).into()))
}
