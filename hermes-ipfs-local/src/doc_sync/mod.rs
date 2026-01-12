//! IPFS document synchronization module.

mod envelope;
mod state_machine;

pub mod payload;
pub mod syn_payload;
pub mod timers;

use ed25519_dalek::VerifyingKey;
pub use envelope::{Envelope, EnvelopePayload};
use minicbor::{Decode, Encode, Encoder, decode, encode::Write};
pub use state_machine::{StateMachine, StateSnapshot, SyncState};

use crate::constant::{CID_DIGEST_SIZE, CID_VERSION, CODEC_CBOR, MULTIHASH_SHA256};

/// Validates CID according to Doc Sync specification constraints.
fn validate_cid(cid: &crate::Cid) -> bool {
    cid.version() as u8 == CID_VERSION
        && cid.codec() == u64::from(CODEC_CBOR)
        && cid.hash().code() == u64::from(MULTIHASH_SHA256)
        && cid.hash().digest().len() == usize::from(CID_DIGEST_SIZE)
}

/// Ed25519 public key instance.
/// Wrapper over `ed25519_dalek::VerifyingKey`.
#[derive(Clone, Debug, PartialEq, Eq, Copy)]
pub struct PublicKey(VerifyingKey);

impl<C> Encode<C> for PublicKey {
    fn encode<W: Write>(
        &self,
        e: &mut Encoder<W>,
        _ctx: &mut C,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        e.bytes(self.0.as_bytes())?;
        Ok(())
    }
}

impl<'b, C> Decode<'b, C> for PublicKey {
    fn decode(
        d: &mut minicbor::Decoder<'b>,
        _ctx: &mut C,
    ) -> Result<Self, minicbor::decode::Error> {
        VerifyingKey::try_from(d.bytes()?)
            .map_err(|err| {
                minicbor::decode::Error::message(format!("error during PublicKey decode: {err}"))
            })
            .map(PublicKey)
    }
}

impl TryFrom<[u8; 32]> for PublicKey {
    type Error = ed25519_dalek::SignatureError;

    fn try_from(bytes: [u8; 32]) -> Result<Self, Self::Error> {
        let vk = VerifyingKey::from_bytes(&bytes)?;
        Ok(PublicKey(vk))
    }
}

/// Ed25519 signature instance.
/// Wrapper over `ed25519_dalek::Signature`.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Signature(ed25519_dalek::Signature);

impl<C> Encode<C> for Signature {
    fn encode<W: Write>(
        &self,
        e: &mut Encoder<W>,
        _ctx: &mut C,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        e.bytes(&self.0.to_bytes())?;
        Ok(())
    }
}

impl<'b, C> Decode<'b, C> for Signature {
    fn decode(
        d: &mut minicbor::Decoder<'b>,
        _ctx: &mut C,
    ) -> Result<Self, minicbor::decode::Error> {
        ed25519_dalek::Signature::try_from(d.bytes()?)
            .map_err(|err| {
                minicbor::decode::Error::message(format!("error during Signature decode: {err}"))
            })
            .map(Signature)
    }
}

/// Blake3-256 hash instance.
/// Wrapper over `blake3::Hash`
#[derive(Clone, Debug, PartialEq, Eq, Copy)]
pub struct Blake3256(blake3::Hash);

impl<C> Encode<C> for Blake3256 {
    fn encode<W: Write>(
        &self,
        e: &mut Encoder<W>,
        _ctx: &mut C,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        e.bytes(self.0.as_bytes())?;
        Ok(())
    }
}

impl<'b, C> Decode<'b, C> for Blake3256 {
    fn decode(
        d: &mut minicbor::Decoder<'b>,
        _ctx: &mut C,
    ) -> Result<Self, minicbor::decode::Error> {
        let b: [u8; 32] = d
            .bytes()?
            .try_into()
            .map_err(|_| decode::Error::message("Invalid Blake3256 length").at(d.position()))?;
        Ok(Blake3256(blake3::Hash::from(b)))
    }
}

impl From<[u8; 32]> for Blake3256 {
    fn from(bytes: [u8; 32]) -> Self {
        Blake3256(blake3::Hash::from(bytes))
    }
}
