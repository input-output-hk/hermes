//! Doc Sync message payloads

use bytemuck::{TransparentWrapper, TransparentWrapperAlloc as _};
use catalyst_types::uuid::{self, UuidV7};
use minicbor::{
    Decode, Decoder, Encode, Encoder, decode,
    encode::{self, Write},
};

use crate::{Cid, doc_sync::Blake3256};

/// Encoding wrapper over [`ipld_core::cid::Cid`].
#[derive(Copy, Clone, TransparentWrapper)]
#[repr(transparent)]
struct CborCid(Cid);

impl<C> Encode<C> for CborCid {
    fn encode<W: Write>(
        &self,
        e: &mut Encoder<W>,
        _: &mut C,
    ) -> Result<(), encode::Error<W::Error>> {
        if super::validate_cid(&self.0) {
            e.bytes(&self.0.to_bytes())?.ok()
        } else {
            Err(encode::Error::message("CID not supported by Doc Sync"))
        }
    }
}

impl<C> Decode<'_, C> for CborCid {
    fn decode(
        d: &mut Decoder<'_>,
        _: &mut C,
    ) -> Result<Self, decode::Error> {
        d.bytes()
            .and_then(|bytes| {
                Cid::try_from(bytes)
                    .map_err(|err| minicbor::decode::Error::custom(err).at(d.position()))
            })
            .and_then(|cid| {
                if super::validate_cid(&cid) {
                    Ok(cid)
                } else {
                    Err(decode::Error::message("CID not supported by Doc Sync").at(d.position()))
                }
            })
            .map(Self)
    }
}

/// Numeric keys of the payload map.
#[derive(Copy, Clone, PartialEq, derive_more::TryFrom)]
#[try_from(repr)]
#[repr(u8)]
enum NumericKeys {
    Root = 1,
    Count = 2,
    Docs = 3,
    Manifest = 4,
    Ttl = 5,
    InReplyTo = 6,
}

impl<C> Encode<C> for NumericKeys {
    fn encode<W: Write>(
        &self,
        e: &mut Encoder<W>,
        _: &mut C,
    ) -> Result<(), encode::Error<W::Error>> {
        e.u8(*self as u8)?.ok()
    }
}

impl<C> Decode<'_, C> for NumericKeys {
    fn decode(
        d: &mut Decoder<'_>,
        _: &mut C,
    ) -> Result<Self, decode::Error> {
        d.u8().and_then(|repr| {
            Self::try_from(repr).map_err(|err| decode::Error::custom(err).at(d.position()))
        })
    }
}

/// Common fields of `.diff` and `.new` message payload maps.
#[derive(Copy, Clone)]
pub struct CommonFields {
    /// Root of the senders SMT with these docs added.
    pub root: Blake3256,
    /// Count of the number of docs in the senders.
    pub count: u64,
    /// Included if this is a reply to a `.syn` topic.
    pub in_reply_to: Option<UuidV7>,
}

impl CommonFields {
    /// Inclusive upper bound on encoded field count.
    const MAX_NUM_FIELDS: u64 = 3;

    /// Counts fields excluding ones that won't be encoded (e.g. being empty).
    fn num_fields(&self) -> u64 {
        if self.in_reply_to.is_none() { 2 } else { 3 }
    }
}

/// Encodes [`CommonFields::root`] key-value pair.
fn encode_root<W: Write>(
    root: &Blake3256,
    e: &mut Encoder<W>,
) -> Result<(), encode::Error<W::Error>> {
    e.encode(NumericKeys::Root)?.encode(root)?.ok()
}

/// Encodes [`CommonFields::count`] key-value pair.
fn encode_count<W: Write>(
    count: u64,
    e: &mut Encoder<W>,
) -> Result<(), encode::Error<W::Error>> {
    e.encode(NumericKeys::Count)?.u64(count)?.ok()
}

/// Encodes [`CommonFields::in_reply_to`] key-value pair.
fn encode_in_reply_to<W: Write>(
    in_reply_to: Option<&UuidV7>,
    e: &mut Encoder<W>,
) -> Result<(), encode::Error<W::Error>> {
    if let Some(uuid) = in_reply_to {
        e.encode(NumericKeys::InReplyTo)?
            .encode_with(uuid, &mut uuid::CborContext::Tagged)?
            .ok()
    } else {
        Ok(())
    }
}

/// Decodes [`CommonFields::root`] key-value pair.
fn decode_root(d: &mut Decoder<'_>) -> Result<Blake3256, decode::Error> {
    if d.decode::<NumericKeys>()
        .is_ok_and(|key| matches!(key, NumericKeys::Root))
    {
        d.decode()
    } else {
        Err(decode::Error::message("Expected `root` key").at(d.position()))
    }
}

/// Decodes [`CommonFields::count`] key-value pair.
fn decode_count(d: &mut Decoder<'_>) -> Result<u64, decode::Error> {
    if d.decode::<NumericKeys>()
        .is_ok_and(|key| matches!(key, NumericKeys::Count))
    {
        d.u64()
    } else {
        Err(decode::Error::message("Expected `count` key").at(d.position()))
    }
}

/// Decodes [`CommonFields::in_reply_to`] key-value pair.
fn decode_in_reply_to(d: &mut Decoder<'_>) -> Result<Option<UuidV7>, decode::Error> {
    if d.decode::<NumericKeys>()
        .is_ok_and(|key| matches!(key, NumericKeys::InReplyTo))
    {
        d.decode_with(&mut uuid::CborContext::Tagged).map(Some)
    } else {
        Ok(None)
    }
}

/// Document dissemination body defined by CDDL spec.
///
/// Enum variants represent possible field combinations of the encoded map.
pub enum DocumentDisseminationBody {
    /// Encoding variant with `docs` field.
    Docs {
        /// Common fields among encoding variants.
        common_fields: CommonFields,
        /// List of `CIDv1` Documents.
        docs: Vec<Cid>,
    },
    /// Encoding variant with `manifest` and `ttl` fields.
    Manifest {
        /// Common fields among encoding variants.
        common_fields: CommonFields,
        /// `CIDv1` of a Manifest of Documents.
        manifest: Cid,
        /// How long the Manifest can be expected to be.
        ttl: u64,
    },
}

impl DocumentDisseminationBody {
    /// Inclusive upper bound on encoded field count.
    const MAX_NUM_FIELDS: u64 = CommonFields::MAX_NUM_FIELDS.saturating_add(2);

    /// Counts fields excluding ones that won't be encoded (e.g. being empty).
    fn num_fields(&self) -> u64 {
        match self {
            DocumentDisseminationBody::Docs { common_fields, .. } => {
                common_fields.num_fields().saturating_add(1)
            },
            DocumentDisseminationBody::Manifest { common_fields, .. } => {
                common_fields.num_fields().saturating_add(2)
            },
        }
    }

    /// Returns `true` if `in_reply_to` field is present.
    fn contains_in_reply_to(&self) -> bool {
        matches!(
            self,
            Self::Docs {
                common_fields: CommonFields {
                    in_reply_to: Some(_),
                    ..
                },
                ..
            } | Self::Manifest {
                common_fields: CommonFields {
                    in_reply_to: Some(_),
                    ..
                },
                ..
            }
        )
    }
}

/// Encodes `docs` field of [`DocumentDisseminationBody::Docs`].
fn encode_docs<W: Write>(
    docs: &[Cid],
    e: &mut Encoder<W>,
) -> Result<(), encode::Error<W::Error>> {
    e.encode(NumericKeys::Docs)?
        .encode(CborCid::wrap_slice(docs))?
        .ok()
}

/// Encodes `manifest` field of [`DocumentDisseminationBody::Manifest`].
fn encode_manifest<W: Write>(
    manifest: &Cid,
    e: &mut Encoder<W>,
) -> Result<(), encode::Error<W::Error>> {
    e.encode(NumericKeys::Manifest)?
        .encode(CborCid::wrap_ref(manifest))?
        .ok()
}

/// Encodes `ttl` field of [`DocumentDisseminationBody::Manifest`].
fn encode_ttl<W: Write>(
    ttl: u64,
    e: &mut Encoder<W>,
) -> Result<(), encode::Error<W::Error>> {
    e.encode(NumericKeys::Ttl)?.u64(ttl)?.ok()
}

/// Helper struct representing [`DocumentDisseminationBody`] variants.
enum DocumentDisseminationBodyKind {
    /// Corresponds to [`DocumentDisseminationBody::Docs`].
    Docs,
    /// Corresponds to [`DocumentDisseminationBody::Manifest`].
    Manifest,
}

impl DocumentDisseminationBodyKind {
    /// Returns which variant of [`DocumentDisseminationBody`]
    /// does the remainder of the encoded fields correspond to.
    fn probe(d: &mut Decoder<'_>) -> Result<Self, decode::Error> {
        d.probe().decode::<NumericKeys>().and_then(|key| {
            match key {
                NumericKeys::Docs => Ok(Self::Docs),
                NumericKeys::Manifest => Ok(Self::Manifest),
                _ => {
                    Err(minicbor::decode::Error::message(
                        "Expected either `docs` or `manifest` field",
                    )
                    .at(d.position()))
                },
            }
        })
    }
}

/// Decodes `docs` field of [`DocumentDisseminationBody::Docs`].
fn decode_docs(d: &mut Decoder<'_>) -> Result<Vec<Cid>, decode::Error> {
    /// Field value is not allowed to exceed 1 MiB.
    const MAX_SIZE: usize = usize::pow(2, 20);

    if d.decode::<NumericKeys>()
        .is_ok_and(|key| matches!(key, NumericKeys::Docs))
    {
        let size = {
            let mut probe = d.probe();
            probe.skip()?;
            probe.position()
        }
        .saturating_sub(d.position());

        if size > MAX_SIZE {
            Err(decode::Error::message("Value of `docs` field is too large"))?;
        }

        d.decode::<Vec<CborCid>>().map(CborCid::peel_vec)
    } else {
        Err(decode::Error::message("Expected `docs` key").at(d.position()))
    }
}

/// Decodes `manifest` field of [`DocumentDisseminationBody::Manifest`].
fn decode_manifest(d: &mut Decoder<'_>) -> Result<Cid, decode::Error> {
    if d.decode::<NumericKeys>()
        .is_ok_and(|key| matches!(key, NumericKeys::Manifest))
    {
        d.decode::<CborCid>().map(CborCid::peel)
    } else {
        Err(decode::Error::message("Expected `manifest` key").at(d.position()))
    }
}

/// Encodes `ttl` field of [`DocumentDisseminationBody::Manifest`].
fn decode_ttl(d: &mut Decoder<'_>) -> Result<u64, decode::Error> {
    if d.decode::<NumericKeys>()
        .is_ok_and(|key| matches!(key, NumericKeys::Ttl))
    {
        d.u64()
    } else {
        Err(decode::Error::message("Expected `ttl` key").at(d.position()))
    }
}

impl<C> Encode<C> for DocumentDisseminationBody {
    fn encode<W: Write>(
        &self,
        e: &mut Encoder<W>,
        _: &mut C,
    ) -> Result<(), encode::Error<W::Error>> {
        e.map(self.num_fields())?;
        match self {
            DocumentDisseminationBody::Docs {
                common_fields,
                docs,
            } => {
                encode_root(&common_fields.root, e)?;
                encode_count(common_fields.count, e)?;
                encode_docs(docs, e)?;
                // Encoding last to maintain deterministic cbor key ordering.
                encode_in_reply_to(common_fields.in_reply_to.as_ref(), e)?;
            },
            DocumentDisseminationBody::Manifest {
                common_fields,
                manifest,
                ttl,
            } => {
                encode_root(&common_fields.root, e)?;
                encode_count(common_fields.count, e)?;
                encode_manifest(manifest, e)?;
                encode_ttl(*ttl, e)?;
                // Encoding last to maintain deterministic cbor key ordering.
                encode_in_reply_to(common_fields.in_reply_to.as_ref(), e)?;
            },
        }
        Ok(())
    }
}

impl<C> Decode<'_, C> for DocumentDisseminationBody {
    fn decode(
        d: &mut Decoder<'_>,
        _: &mut C,
    ) -> Result<Self, decode::Error> {
        let map_len = d.map()?.ok_or_else(|| {
            decode::Error::message("Expected definite-sized map").at(d.position())
        })?;

        if map_len > Self::MAX_NUM_FIELDS {
            Err(decode::Error::message("Too many fields in a map").at(d.position()))?;
        }

        let root = decode_root(d)?;
        let count = decode_count(d)?;

        match DocumentDisseminationBodyKind::probe(d)? {
            DocumentDisseminationBodyKind::Docs => {
                let docs = decode_docs(d)?;
                let in_reply_to = decode_in_reply_to(d)?;

                Ok(Self::Docs {
                    common_fields: CommonFields {
                        root,
                        count,
                        in_reply_to,
                    },
                    docs,
                })
            },
            DocumentDisseminationBodyKind::Manifest => {
                let manifest = decode_manifest(d)?;
                let ttl = decode_ttl(d)?;
                let in_reply_to = decode_in_reply_to(d)?;

                Ok(Self::Manifest {
                    common_fields: CommonFields {
                        root,
                        count,
                        in_reply_to,
                    },
                    manifest,
                    ttl,
                })
            },
        }
    }
}

/// `.new` message payload.
#[derive(Encode, derive_more::Deref, derive_more::Into)]
#[cbor(transparent)]
pub struct New(#[n(0)] DocumentDisseminationBody);

impl<C> Decode<'_, C> for New {
    fn decode(
        d: &mut Decoder<'_>,
        ctx: &mut C,
    ) -> Result<Self, decode::Error> {
        let body = DocumentDisseminationBody::decode(d, ctx)?;

        let contains_in_reply_to = body.contains_in_reply_to();

        if contains_in_reply_to {
            Err(decode::Error::message("Expected no `in_reply_to` field"))
        } else {
            Ok(Self(body))
        }
    }
}

impl TryFrom<DocumentDisseminationBody> for New {
    type Error = anyhow::Error;

    fn try_from(value: DocumentDisseminationBody) -> Result<Self, Self::Error> {
        if value.contains_in_reply_to() {
            Err(anyhow::Error::msg("Expected no `in_reply_to` field"))
        } else {
            Ok(Self(value))
        }
    }
}

/// `.diff` message payload.
#[derive(Decode, Encode, derive_more::Deref, derive_more::From, derive_more::Into)]
#[cbor(transparent)]
pub struct Diff(#[n(0)] DocumentDisseminationBody);

#[cfg(test)]
#[allow(missing_docs)]
mod tests {
    use anyhow::anyhow;

    use super::{Diff, New};

    #[test]
    fn docs() -> anyhow::Result<()> {
        let docs = body::to_cbor(&[body::root_hash, body::count, body::docs])?;

        let new_docs_decoded = minicbor::decode::<New>(&docs)?;
        let diff_docs_decoded = minicbor::decode::<Diff>(&docs)?;

        let new_docs_encoded = minicbor::to_vec(new_docs_decoded)?;
        let diff_docs_encoded = minicbor::to_vec(diff_docs_decoded)?;

        anyhow::ensure!(new_docs_encoded == docs, "{docs:?}");
        anyhow::ensure!(diff_docs_encoded == docs, "{diff_docs_encoded:?}");

        Ok(())
    }

    #[test]
    fn docs_in_reply_to() -> anyhow::Result<()> {
        let docs_in_reply_to =
            body::to_cbor(&[body::root_hash, body::count, body::docs, body::in_reply_to])?;

        minicbor::decode::<New>(&docs_in_reply_to)
            .err()
            .ok_or_else(|| anyhow!(".new should not decode with in_reply_to"))?;
        let diff_docs_in_reply_to_decoded = minicbor::decode::<Diff>(&docs_in_reply_to)?;

        let diff_docs_in_reply_to_encoded = minicbor::to_vec(diff_docs_in_reply_to_decoded)?;

        anyhow::ensure!(
            diff_docs_in_reply_to_encoded == docs_in_reply_to,
            "{diff_docs_in_reply_to_encoded:?}"
        );

        Ok(())
    }

    #[test]
    fn manifest() -> anyhow::Result<()> {
        let manifest = body::to_cbor(&[body::root_hash, body::count, body::manifest, body::ttl])?;

        let new_manifest_decoded = minicbor::decode::<New>(&manifest)?;
        let diff_manifest_decoded = minicbor::decode::<Diff>(&manifest)?;

        let new_manifest_encoded = minicbor::to_vec(new_manifest_decoded)?;
        let diff_manifest_encoded = minicbor::to_vec(diff_manifest_decoded)?;

        anyhow::ensure!(new_manifest_encoded == manifest, "{new_manifest_encoded:?}");
        anyhow::ensure!(
            diff_manifest_encoded == manifest,
            "{diff_manifest_encoded:?}"
        );

        Ok(())
    }

    #[test]
    fn manifest_in_reply_to() -> anyhow::Result<()> {
        let manifest_in_reply_to = body::to_cbor(&[
            body::root_hash,
            body::count,
            body::manifest,
            body::ttl,
            body::in_reply_to,
        ])?;

        minicbor::decode::<New>(&manifest_in_reply_to)
            .err()
            .ok_or_else(|| anyhow!(".new should not decode with in_reply_to"))?;

        let diff_manifest_in_reply_to_decoded = minicbor::decode::<Diff>(&manifest_in_reply_to)?;

        let diff_manifest_in_reply_to_encoded =
            minicbor::to_vec(diff_manifest_in_reply_to_decoded)?;

        anyhow::ensure!(
            diff_manifest_in_reply_to_encoded == manifest_in_reply_to,
            "{diff_manifest_in_reply_to_encoded:?}"
        );

        Ok(())
    }

    /// Bodies used as fixed test data.
    mod body {
        use minicbor::data::{Tag, Token};

        use crate::constant::{CID_DIGEST_SIZE, CODEC_CBOR, MULTIHASH_SHA256, PROTOCOL_VERSION};

        // Generates a valid Doc Sync `CID` (according to the spec).
        const fn generate_cid(seed: u8) -> [u8; 36] {
            let prefix = [
                PROTOCOL_VERSION,
                CODEC_CBOR,
                MULTIHASH_SHA256,
                CID_DIGEST_SIZE,
            ];
            let mut ret = [seed; 36];
            ret.split_at_mut(prefix.len()).0.copy_from_slice(&prefix);
            ret
        }

        // Generates a valid Doc Sync `UUID` (according to the spec).
        const fn generate_uuid(seed: u8) -> [u8; 16] {
            // Arbitrary valid `UuidV7` prefix.
            const PREFIX: [u8; 9] = [0, 0, 0, 0, 0, 0, 7 << 4, 0, 9 << 4];
            let mut ret = [seed; 16];
            ret.split_at_mut(PREFIX.len()).0.copy_from_slice(&PREFIX);
            ret
        }

        pub const fn root_hash() -> &'static [Token<'static>] {
            &[Token::U8(1), Token::Bytes(&[32; 32])]
        }

        pub const fn count() -> &'static [Token<'static>] {
            &[Token::U8(2), Token::U8(3)]
        }

        pub const fn docs() -> &'static [Token<'static>] {
            &[
                Token::U8(3),
                Token::Array(3),
                Token::Bytes(&const { generate_cid(1) }),
                Token::Bytes(&const { generate_cid(2) }),
                Token::Bytes(&const { generate_cid(3) }),
            ]
        }

        pub const fn manifest() -> &'static [Token<'static>] {
            &[Token::U8(4), Token::Bytes(&const { generate_cid(4) })]
        }

        pub const fn ttl() -> &'static [Token<'static>] {
            &[Token::U8(5), Token::U64(u64::MAX)]
        }

        pub const fn in_reply_to() -> &'static [Token<'static>] {
            &[
                Token::U8(6),
                Token::Tag(const { Tag::new(37) }),
                Token::Bytes(&const { generate_uuid(1) }),
            ]
        }

        /// Constructs a body from its fields.
        pub fn to_cbor(fields: &[fn() -> &'static [Token<'static>]]) -> anyhow::Result<Vec<u8>> {
            let mut buf = minicbor::Encoder::new(vec![]);
            buf.map(u64::try_from(fields.len())?)?;
            fields
                .iter()
                .flat_map(|f| f())
                .try_for_each(|token| buf.encode(token)?.ok())?;
            Ok(buf.into_writer())
        }
    }
}
