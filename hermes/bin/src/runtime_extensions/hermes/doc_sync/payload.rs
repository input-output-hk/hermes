//! Doc Sync message payloads

/*
; Payload body fits within the Common Message Envelope
payload-body = doc-dissemination-body

; numeric keys (shared by .new and .dif)
root = 1
count = 2
docs = 3
manifest = 4
ttl = 5
in_reply_to = 6

common-fields = (
    root => root-hash,     ; Root of the senders SMT with these docs added.
    count => uint,         ; Count of the number of docs in the senders Merkle Tree.
    ? in_reply_to => uuid, ; Included if this is a reply to a `.syn` topic message.
)

doc-dissemination-body = ({
    common-fields,        ; All fields common to doc lists or doc manifests
    docs => [* cidv1]     ; List of CIDv1 Documents
} / {
    common-fields,        ; All fields common to doc lists or doc manifests
    manifest => cidv1,    ; CIDv1 of a Manifest of Documents
    ttl => uint           ; How long the Manifest can be expected to be pinned by the sender.
})

; self-contained types
blake3-256 = bytes .size 32 ; BLAKE3-256 output
root-hash = blake3-256      ; Root hash of the Sparse Merkle Tree
cidv1 = bytes .size (36..40)  ; CIDv1 (binary); multihash MUST be sha2-256 (32-byte digest)
uuid = #6.37(bytes .size 16) ; UUIDv7
*/

use bytemuck::{TransparentWrapper, TransparentWrapperAlloc as _};
use catalyst_types::uuid::{self, UuidV7};
use derive_more::{Deref, From, Into, TryFrom};
use hermes_ipfs::Cid;
use minicbor::{
    Decode, Decoder, Encode, Encoder, decode,
    encode::{self, Write},
};

/// Encoding wrapper over [`hermes_ipfs::Cid`].
#[derive(Copy, Clone, TransparentWrapper)]
#[repr(transparent)]
struct CborCid(Cid);

impl<C> Encode<C> for CborCid {
    fn encode<W: Write>(
        &self,
        e: &mut Encoder<W>,
        _: &mut C,
    ) -> Result<(), encode::Error<W::Error>> {
        e.bytes(&self.0.to_bytes())?.ok()
    }
}

impl<C> Decode<'_, C> for CborCid {
    fn decode(
        d: &mut Decoder<'_>,
        _: &mut C,
    ) -> Result<Self, decode::Error> {
        d.bytes()
            .and_then(|bytes| {
                hermes_ipfs::Cid::try_from(bytes)
                    .map_err(|err| minicbor::decode::Error::custom(err).at(d.position()))
            })
            .map(Self)
    }
}

#[derive(Copy, Clone, TryFrom, PartialEq)]
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

pub type RootHash = [u8; 32];

#[derive(Copy, Clone, Default)]
pub struct CommonFields {
    pub root: RootHash,
    pub count: u64,
    pub in_reply_to: Option<UuidV7>,
}

impl CommonFields {
    const MAX_NUM_FIELDS: u64 = 3;

    fn num_fields(&self) -> u64 {
        if self.in_reply_to.is_none() { 2 } else { 3 }
    }
}

fn encode_root<W: Write>(
    root: &RootHash,
    e: &mut Encoder<W>,
) -> Result<(), encode::Error<W::Error>> {
    e.encode(NumericKeys::Root)?.bytes(root)?.ok()
}

fn encode_count<W: Write>(
    count: u64,
    e: &mut Encoder<W>,
) -> Result<(), encode::Error<W::Error>> {
    e.encode(NumericKeys::Count)?.u64(count)?.ok()
}

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

fn decode_root(d: &mut Decoder<'_>) -> Result<RootHash, decode::Error> {
    if d.decode::<NumericKeys>()
        .is_ok_and(|key| matches!(key, NumericKeys::Root))
    {
        d.decode()
    } else {
        Err(decode::Error::message("Expected `root` key").at(d.position()))
    }
}

fn decode_count(d: &mut Decoder<'_>) -> Result<u64, decode::Error> {
    if d.decode::<NumericKeys>()
        .is_ok_and(|key| matches!(key, NumericKeys::Count))
    {
        d.decode()
    } else {
        Err(decode::Error::message("Expected `count` key").at(d.position()))
    }
}

fn decode_in_reply_to(d: &mut Decoder<'_>) -> Result<Option<UuidV7>, decode::Error> {
    if d.decode::<NumericKeys>()
        .is_ok_and(|key| matches!(key, NumericKeys::InReplyTo))
    {
        d.decode_with(&mut uuid::CborContext::Tagged).map(Some)
    } else {
        Ok(None)
    }
}

pub enum DocumentDisseminationBody {
    Docs {
        common_fields: CommonFields,
        docs: Vec<Cid>,
    },
    Manifest {
        common_fields: CommonFields,
        manifest: Cid,
        ttl: u64,
    },
}

impl DocumentDisseminationBody {
    const MAX_NUM_FIELDS: u64 = CommonFields::MAX_NUM_FIELDS.saturating_add(2);

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

    fn contains_in_reply_to(&self) -> bool {
        !matches!(
            self,
            Self::Docs {
                common_fields: CommonFields {
                    in_reply_to: None,
                    ..
                },
                ..
            } | Self::Manifest {
                common_fields: CommonFields {
                    in_reply_to: None,
                    ..
                },
                ..
            }
        )
    }
}

fn encode_docs<W: Write>(
    docs: &[Cid],
    e: &mut Encoder<W>,
) -> Result<(), encode::Error<W::Error>> {
    e.encode(NumericKeys::Docs)?
        .encode(&CborCid::wrap_slice(docs))?
        .ok()
}

fn encode_manifest<W: Write>(
    manifest: &Cid,
    e: &mut Encoder<W>,
) -> Result<(), encode::Error<W::Error>> {
    e.encode(NumericKeys::Manifest)?
        .encode(&CborCid::wrap_ref(manifest))?
        .ok()
}

fn encode_ttl<W: Write>(
    ttl: u64,
    e: &mut Encoder<W>,
) -> Result<(), encode::Error<W::Error>> {
    e.encode(NumericKeys::Ttl)?.u64(ttl)?.ok()
}

enum DocumentDisseminationBodyKind {
    Docs,
    Manifest,
}

impl DocumentDisseminationBodyKind {
    fn probe(d: &mut Decoder<'_>) -> Result<Self, decode::Error> {
        d.probe().decode::<NumericKeys>().and_then(|key| match key {
            NumericKeys::Docs => Ok(Self::Docs),
            NumericKeys::Manifest => Ok(Self::Manifest),
            _ => Err(minicbor::decode::Error::message(
                "Expected either `docs` or `manifest` field",
            )
            .at(d.position())),
        })
    }
}

fn decode_docs(d: &mut Decoder<'_>) -> Result<Vec<Cid>, decode::Error> {
    if d.decode::<NumericKeys>()
        .is_ok_and(|key| matches!(key, NumericKeys::Docs))
    {
        d.decode::<Vec<CborCid>>().map(CborCid::peel_vec)
    } else {
        Err(decode::Error::message("Expected `docs` key").at(d.position()))
    }
}

fn decode_manifest(d: &mut Decoder<'_>) -> Result<Cid, decode::Error> {
    if d.decode::<NumericKeys>()
        .is_ok_and(|key| matches!(key, NumericKeys::Manifest))
    {
        d.decode::<CborCid>().map(CborCid::peel)
    } else {
        Err(decode::Error::message("Expected `manifest` key").at(d.position()))
    }
}

fn decode_ttl(d: &mut Decoder<'_>) -> Result<u64, decode::Error> {
    if d.decode::<NumericKeys>()
        .is_ok_and(|key| matches!(key, NumericKeys::Ttl))
    {
        d.decode()
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
                encode_docs(&docs, e)?;
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
                encode_manifest(&manifest, e)?;
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

#[derive(Deref, Encode, Into)]
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

#[derive(Deref, Decode, Encode, From, Into)]
#[cbor(transparent)]
pub struct Diff(#[n(0)] DocumentDisseminationBody);
