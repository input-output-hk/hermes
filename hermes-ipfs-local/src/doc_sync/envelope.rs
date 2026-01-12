//! Document synchronization envelope module.
//!
//! Implements the envelope layout described in the IPFS Document Sync
//! specification: an inner `[peer, seq, ver, payload, signature]` array,
//! signed as-is, and wrapped inside an outer CBOR `bstr` before it is sent.

use std::convert::Infallible;

use anyhow::{Context, ensure};
use catalyst_types::uuid::{self, CborContext, UuidV7};
use ed25519_dalek::VerifyingKey;
use minicbor::{Decode, Encode, Encoder, data::Type, encode::Write};

use crate::{
    constant::PROTOCOL_VERSION,
    doc_sync::{PublicKey, Signature},
};

/// The unsigned portion of the message envelope.
/// This structure corresponds to the **signature input** array:
/// `[peer, seq, ver, payload]`.
///
/// The entire array is deterministically CBOR encoded and then signed to create the final
/// `signed-payload`.
pub struct EnvelopePayload {
    /// Matches sender's Peer ID in IPFS Network.
    /// Peer ID can be derived from this public key.
    peer: PublicKey,
    /// Unique nonce and timestamp.
    /// Prevents and helps detect message duplication.
    seq: UuidV7,
    /// Protocol version number.
    /// This should be `1` for the current specification.
    ver: u64,
    /// Deterministically-encoded CBOR map (`payload-body`).
    payload: Vec<u8>,
}

impl EnvelopePayload {
    /// Create new instance of `EnvelopePayload`.
    ///
    /// # Errors
    ///
    /// Returns an error when `payload` is not a single CBOR map.
    pub fn new(
        peer: ed25519_dalek::VerifyingKey,
        payload: Vec<u8>,
    ) -> anyhow::Result<Self> {
        ensure_payload_body(&payload)?;
        Ok(Self {
            peer: PublicKey(peer),
            seq: UuidV7::new(),
            ver: PROTOCOL_VERSION.into(),
            payload,
        })
    }

    /// Returns the peer verifying key reference.
    #[must_use]
    pub fn peer(&self) -> &VerifyingKey {
        &self.peer.0
    }

    /// Returns the `UuidV7` sequence value.
    #[must_use]
    pub fn seq(&self) -> uuid::Uuid {
        self.seq.uuid()
    }

    /// Returns the encoded payload-body bytes.
    #[must_use]
    pub fn payload_bytes(&self) -> &[u8] {
        &self.payload
    }

    /// Returns the decoded payload body.
    ///
    /// # Errors
    ///
    /// Returns an error if the payload bytes cannot be decoded into the requested type.
    pub fn payload<T: for<'a> Decode<'a, ()>>(&self) -> Result<(), minicbor::decode::Error> {
        minicbor::decode(self.payload_bytes())
    }

    /// Returns CBOR bytes for `[peer, seq, ver, payload]`.
    ///
    /// These are the bytes the spec signs over.
    ///
    /// # Errors
    ///
    /// Returns an error if encoding fails (should not happen with `Vec<u8>` writers).
    pub fn to_bytes(&self) -> Result<Vec<u8>, minicbor::encode::Error<Infallible>> {
        minicbor::to_vec(self)
    }

    /// Decodes `[peer, seq, ver, payload]` from the signed payload array.
    fn decode_from_signed<C>(
        decoder: &mut minicbor::Decoder<'_>,
        ctx: &mut C,
    ) -> Result<Self, minicbor::decode::Error> {
        let peer: PublicKey = decoder.decode_with(ctx)?;
        let seq: UuidV7 = decoder.decode_with(&mut CborContext::Tagged)?;
        let ver = decoder.u64()?;

        if ver != u64::from(PROTOCOL_VERSION) {
            return Err(minicbor::decode::Error::message(format!(
                "unsupported protocol version: {ver}"
            )));
        }

        let datatype = decoder.datatype()?;
        if !matches!(datatype, Type::Map) {
            return Err(minicbor::decode::Error::message(
                "payload-body must be a CBOR map",
            ));
        }

        let start = decoder.position();
        decoder.skip()?;
        let end = decoder.position();
        let input = decoder.input();
        let payload_slice = input.get(start..end).ok_or_else(|| {
            minicbor::decode::Error::message("payload-body slice exceeds decoder input")
        })?;

        Ok(Self {
            peer,
            seq,
            ver,
            payload: payload_slice.to_vec(),
        })
    }
}

impl<C> Encode<C> for EnvelopePayload {
    fn encode<W: Write>(
        &self,
        e: &mut Encoder<W>,
        ctx: &mut C,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        e.array(4)?;
        e.encode_with(self.peer, ctx)?
            .encode_with(self.seq, &mut CborContext::Tagged)?
            .u64(self.ver)?;
        <W as Write>::write_all(e.writer_mut(), &self.payload)
            .map_err(minicbor::encode::Error::write)?;
        Ok(())
    }
}

impl<'b, C> Decode<'b, C> for EnvelopePayload {
    fn decode(
        d: &mut minicbor::Decoder<'b>,
        ctx: &mut C,
    ) -> Result<Self, minicbor::decode::Error> {
        let len = d.array()?;
        match len {
            Some(4) => {},
            Some(other) => {
                return Err(minicbor::decode::Error::message(format!(
                    "unexpected payload array length: {other}"
                )));
            },
            None => {
                return Err(minicbor::decode::Error::message(
                    "indefinite payload arrays are not supported",
                ));
            },
        }

        let peer: PublicKey = d.decode_with(ctx)?;
        let seq: UuidV7 = d.decode_with(&mut CborContext::Tagged)?;
        let ver = d.u64()?;

        if ver != u64::from(PROTOCOL_VERSION) {
            return Err(minicbor::decode::Error::message(format!(
                "unsupported protocol version: {ver}"
            )));
        }

        let datatype = d.datatype()?;
        if !matches!(datatype, Type::Map) {
            return Err(minicbor::decode::Error::message(
                "payload-body must be a CBOR map",
            ));
        }

        let start = d.position();
        d.skip()?;
        let end = d.position();
        let input = d.input();
        let payload_slice = input.get(start..end).ok_or_else(|| {
            minicbor::decode::Error::message("payload-body slice exceeds decoder input")
        })?;

        Ok(Self {
            peer,
            seq,
            ver,
            payload: payload_slice.to_vec(),
        })
    }
}

/// Helper struct for encoding the inner [peer, seq, ver, payload, sig] array.
struct SignedPayloadView<'a> {
    /// Reference to the envelope payload (providing peer, seq, ver, and body).
    payload: &'a EnvelopePayload,
    /// Reference to the signature verifying the payload.
    signature: &'a Signature,
}

impl<C> Encode<C> for SignedPayloadView<'_> {
    fn encode<W: Write>(
        &self,
        e: &mut Encoder<W>,
        _ctx: &mut C,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        e.array(5)?;
        e.encode(self.payload.peer)?;
        e.encode_with(self.payload.seq, &mut CborContext::Tagged)?;
        e.u64(self.payload.ver)?;
        e.writer_mut()
            .write_all(&self.payload.payload)
            .map_err(minicbor::encode::Error::write)?;
        e.encode(self.signature)?;
        Ok(())
    }
}

/// The final outer message structure.
///
/// `Envelope` owns both the `[peer, seq, ver, payload, signature]` array (which
/// is the signed payload defined by the spec) and the outer “framing” when it
/// is encoded. The `Encode` implementation outputs a CBOR byte string whose
/// content is exactly that five-element array, matching the spec’s wording:
/// “`envelope = bstr .cbor signed-payload`”.
pub struct Envelope {
    /// The inner payload array: `[peer, seq, ver, payload]`.
    /// This is the exact content that is deterministically CBOR encoded and signed.
    payload: EnvelopePayload,
    /// This is the signature computed over the deterministic CBOR bytes of
    /// `self.payload`.
    signature: Signature,
}

impl Envelope {
    /// Creates new doc sync envelope.
    ///
    /// Performs signature validation (step 1 of verification) as per spec.
    ///
    /// # Arguments
    ///
    /// * `payload` - The unsigned `EnvelopePayload`.
    /// * `signature` - `ed25519_dalek::Signature` of provided payload's deterministic
    ///   bytes.
    ///
    /// # Errors
    ///
    /// Returns an error if signature is invalid.
    pub fn new(
        payload: EnvelopePayload,
        signature: ed25519_dalek::Signature,
    ) -> anyhow::Result<Self> {
        payload
            .peer
            .0
            .verify_strict(&payload.to_bytes()?, &signature)?;
        Ok(Self {
            payload,
            signature: Signature(signature),
        })
    }

    /// Returns the inner payload.
    #[must_use]
    pub fn payload(&self) -> &EnvelopePayload {
        &self.payload
    }

    /// Returns the signature over the payload.
    #[must_use]
    pub fn signature(&self) -> &ed25519_dalek::Signature {
        &self.signature.0
    }

    /// Returns the deterministic CBOR `envelope` bstr defined in the spec.
    ///
    /// `envelope = bstr .size (82..1048576) .cbor signed-payload`.
    ///
    /// # Errors
    ///
    /// Returns an error if encoding fails (should not happen with `Vec<u8>` writers).
    pub fn to_bytes(&self) -> Result<Vec<u8>, minicbor::encode::Error<Infallible>> {
        minicbor::to_vec(self)
    }
}

impl<C> Encode<C> for Envelope {
    fn encode<W: Write>(
        &self,
        e: &mut Encoder<W>,
        ctx: &mut C,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        let view = SignedPayloadView {
            payload: &self.payload,
            signature: &self.signature,
        };

        let inner_bytes = minicbor::to_vec_with(&view, ctx)
            .map_err(|e| minicbor::encode::Error::message(e.to_string()))?;

        e.bytes(&inner_bytes)?;
        Ok(())
    }
}

impl<'b, C> Decode<'b, C> for Envelope {
    fn decode(
        d: &mut minicbor::Decoder<'b>,
        ctx: &mut C,
    ) -> Result<Self, minicbor::decode::Error> {
        let signed_payload_bytes = d.bytes()?;
        let mut signed_decoder = minicbor::Decoder::new(signed_payload_bytes);

        let len = signed_decoder.array()?;
        match len {
            Some(5) => {},
            Some(other) => {
                return Err(minicbor::decode::Error::message(format!(
                    "unexpected envelope array length: {other}"
                )));
            },
            None => {
                return Err(minicbor::decode::Error::message(
                    "indefinite envelope arrays are not supported",
                ));
            },
        }

        let payload = EnvelopePayload::decode_from_signed(&mut signed_decoder, ctx)?;
        let signature: Signature = signed_decoder.decode_with(ctx)?;

        Ok(Self { payload, signature })
    }
}

/// Ensures that the provided bytes contain exactly one CBOR map.
fn ensure_payload_body(bytes: &[u8]) -> anyhow::Result<()> {
    let mut decoder = minicbor::Decoder::new(bytes);
    let datatype = decoder
        .datatype()
        .context("failed to inspect payload-body datatype")?;
    ensure!(
        matches!(datatype, Type::Map),
        "payload-body must encode a CBOR map"
    );
    decoder.skip().context("failed to parse payload-body map")?;
    ensure!(
        decoder.position() == bytes.len(),
        "payload-body must contain exactly one CBOR map"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use ed25519_dalek::{Signer, SigningKey};
    use minicbor::Decoder;

    use super::*;

    fn signing_key() -> SigningKey {
        SigningKey::from_bytes(&[7u8; 32])
    }

    fn sample_payload_body() -> Vec<u8> {
        let mut enc = Encoder::new(Vec::new());
        enc.map(1).unwrap();
        enc.str("doc").unwrap();
        enc.bytes(b"cid").unwrap();
        enc.into_writer()
    }

    #[test]
    fn envelope_roundtrip_matches_spec_bstr() {
        let signing_key = signing_key();
        let payload_body = sample_payload_body();
        let payload = EnvelopePayload::new(signing_key.verifying_key(), payload_body.clone())
            .expect("payload");

        let signature = signing_key
            .try_sign(&payload.to_bytes().expect("bytes"))
            .expect("signature");
        let envelope = Envelope::new(payload, signature).expect("envelope");

        let encoded = envelope.to_bytes().expect("encode");

        // Outermost element must be a CBOR byte string.
        let mut outer = Decoder::new(&encoded);
        let signed_payload = outer.bytes().expect("bstr");
        assert_eq!(outer.position(), encoded.len());

        // Inner element must be the signed payload array with five entries.
        let mut inner = Decoder::new(signed_payload);
        let len = inner.array().expect("array");
        assert_eq!(len, Some(5));

        // Decode fully to ensure `Decode` impl works.
        let mut envelope_decoder = Decoder::new(&encoded);
        let decoded: Envelope = envelope_decoder
            .decode_with(&mut CborContext::Tagged)
            .expect("decode");
        assert_eq!(decoded.payload().payload_bytes(), payload_body.as_slice());
    }

    #[test]
    fn decode_rejects_wrong_protocol_version() {
        let signing_key = signing_key();
        let payload_body = sample_payload_body();
        let payload =
            EnvelopePayload::new(signing_key.verifying_key(), payload_body).expect("payload");

        let signature = signing_key
            .try_sign(&payload.to_bytes().expect("bytes"))
            .expect("signature");

        let mut signed = Encoder::new(Vec::new());
        signed.array(5).unwrap();
        signed.encode(payload.peer).unwrap();
        signed
            .encode_with(payload.seq, &mut CborContext::Tagged)
            .unwrap();
        signed.u64(PROTOCOL_VERSION.into() + 1).unwrap();
        <Vec<u8> as Write>::write_all(signed.writer_mut(), &payload.payload).unwrap();
        signed.encode(Signature(signature)).unwrap();

        let mut envelope = Encoder::new(Vec::new());
        envelope.bytes(&signed.into_writer()).unwrap();
        let bytes = envelope.into_writer();

        let mut decoder = Decoder::new(&bytes);
        let result: Result<Envelope, _> = decoder.decode_with(&mut CborContext::Tagged);
        assert!(result.is_err(), "expected version mismatch error");
    }

    #[test]
    fn envelope_new_rejects_bad_signature() {
        let signing_key = signing_key();
        let payload_body = sample_payload_body();
        let payload =
            EnvelopePayload::new(signing_key.verifying_key(), payload_body).expect("payload");

        let mut signature = signing_key
            .try_sign(&payload.to_bytes().expect("bytes"))
            .expect("signature")
            .to_bytes();
        signature[0] ^= 0xFF;
        let signature = ed25519_dalek::Signature::from_bytes(&signature);

        let result = Envelope::new(payload, signature);
        assert!(result.is_err(), "signature validation must fail");
    }

    #[test]
    fn payload_validation_rejects_non_map() {
        let signing_key = signing_key();
        let mut enc = Encoder::new(Vec::new());
        enc.array(1).unwrap();
        enc.u8(42).unwrap();
        let payload_bytes = enc.into_writer();

        let result = EnvelopePayload::new(signing_key.verifying_key(), payload_bytes);
        assert!(result.is_err(), "non-map payload must be rejected");
    }

    #[test]
    fn decode_rejects_missing_outer_bstr() {
        // Attempt to decode the array directly without the outer bstr wrapping
        let signing_key = signing_key();
        let payload_body = sample_payload_body();
        let payload =
            EnvelopePayload::new(signing_key.verifying_key(), payload_body).expect("payload");
        let signature = signing_key
            .try_sign(&payload.to_bytes().expect("bytes"))
            .expect("signature");
        let signature = Signature(signature);

        let view = SignedPayloadView {
            payload: &payload,
            signature: &signature,
        };
        let encoded_array = minicbor::to_vec(&view).unwrap();

        let mut decoder = Decoder::new(&encoded_array);
        // We use () context here for simplicity
        let result: Result<Envelope, _> = decoder.decode_with(&mut ());

        // This should fail because `decode` expects `d.bytes()` (bstr) first,
        // but it will encounter an array tag (0x85).
        assert!(result.is_err(), "must reject content without outer bstr");
    }

    #[test]
    fn decode_rejects_malformed_inner_array_len() {
        let signing_key = signing_key();
        let payload_body = sample_payload_body();
        let payload =
            EnvelopePayload::new(signing_key.verifying_key(), payload_body).expect("payload");

        // Construct a fake array of length 4 instead of 5
        let mut bad_array = Encoder::new(Vec::new());
        bad_array.array(4).unwrap(); // Wrong length
        bad_array.encode(payload.peer).unwrap();
        bad_array
            .encode_with(payload.seq, &mut CborContext::Tagged)
            .unwrap();
        bad_array.u64(PROTOCOL_VERSION.into()).unwrap();
        // Skip payload & signature to force length error or skip signature

        let mut envelope = Encoder::new(Vec::new());
        envelope.bytes(&bad_array.into_writer()).unwrap();
        let bytes = envelope.into_writer();

        let mut decoder = Decoder::new(&bytes);
        let result: Result<Envelope, _> = decoder.decode_with(&mut ());
        assert!(result.is_err(), "must reject wrong array length");
    }

    #[test]
    fn compiles_with_custom_context() {
        // Verify that we can pass a custom struct as context C
        struct MyMetrics {
            _bytes_read: usize,
        }

        let signing_key = signing_key();
        let payload_body = sample_payload_body();
        let payload = EnvelopePayload::new(signing_key.verifying_key(), payload_body.clone())
            .expect("payload");
        let signature = signing_key.sign(&payload.to_bytes().unwrap());
        let envelope = Envelope::new(payload, signature).expect("envelope");

        let bytes = envelope.to_bytes().expect("bytes");

        let mut ctx = MyMetrics { _bytes_read: 0 };
        let mut decoder = Decoder::new(&bytes);

        // This validates that the generic <C> is properly propagated
        let _decoded: Envelope = decoder.decode_with(&mut ctx).expect("decode with context");
    }
}
