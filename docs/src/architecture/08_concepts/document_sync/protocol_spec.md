---
icon: material/file-document-outline
---

# IPFS Document Sync Protocol — Specification (Draft)

## Status

* Stage: PoC draft

* Version: 0.1.0 (wire format frozen per this document; numeric parameters may be tuned)

## Overview

* Purpose: Efficiently synchronize a set of document CIDs across peers using IPFS pub/sub broadcast for announcements and IBLT-based set reconciliation for divergence.

* Design: Append-only document set represented by a Sparse Merkle Tree (SMT) root.
  Three required pub/sub topics per set: `<base>.new`, `<base>.syn`, `<base>.dif`.
  Two optional topics for proofs: `<base>.prv` (proof requests) and `<base>.prf` (proof replies).
* Assumptions (PoC): Honest peers, no privacy, all messages publicly readable.
  All payloads CBOR-encoded in canonical form with strict framing.

## Scope and Goals

* Ensure eventual consistency of document sets across honest peers.

* Minimize pub/sub bandwidth via batched announcements and IBLT-based diffs.
* Idempotent processing of duplicates and replays.
* Generic over `<base>` topic names; multiple sets may run in parallel.

## Non-Goals

* Adversarial hardening (Sybil/DoS resistance, spam prevention).

* Confidentiality or payload encryption.
* Membership/admission control.

## Terminology

* CID: IPFS Content Identifier.

* SMT: Sparse Merkle Tree (append-only presence set over CIDs).
* Root: SMT root hash summarizing the entire set.
* IBLT: Invertible Bloom Lookup Table used for set reconciliation.
* Manifest: IPFS object (by CID) describing a batch of CIDs or a diff.
* UUIDv7: 128-bit, time-ordered unique identifier used as message/correlation id.

## Roles and Entities

* Peer: A participant with an IPFS/libp2p node publishing and subscribing to the three topics for a given `<base>`.

* Set: The document set scoped to `<base>`.

## Protocol Versioning and Negotiation

* Field `ver` within payloads carries protocol version (uint).
  This document defines `ver = 1`.

* Implementations MUST ignore unknown optional fields; unknown critical fields MUST cause rejection.

## Transport Bindings

* Pub/Sub: libp2p gossipsub (via IPFS pubsub).
  All messages are broadcasts on:
  * `<base>.new` (announcements of new CIDs and the sender’s resulting root),
  * `<base>.syn` (solicitations for reconciliation, including the requester’s sketch),
  * `<base>.dif` (reconciliation replies and/or pointers to diff manifests),
  * `<base>.prv` OPTIONAL (requests for SMT inclusion proofs of specific CIDs),
  * `<base>.prf` OPTIONAL (proof replies containing SMT proofs).

* No direct streams are required in this PoC; all reconciliation occurs on pub/sub.

## Topics and Namespacing

* `<base>` is an opaque UTF-8 string under 120 characters, defined by higher-level context.

* Topic semantics are single-purpose: a topic MUST carry only its designated message type.
  Peers MAY drop senders violating this.
* Proof topics are OPTIONAL.
  Topics that require verifiability SHOULD additionally subscribe to `<base>.prv` and `<base>.prf`.

## Message Model

* Framing and Signature Envelope:
  * Each published message item is a CBOR byte string (bstr), whose content is a canonical CBOR array of two elements: `[payload_bstr, signature_bstr]`.
  * `payload_bstr` is a CBOR byte string containing the canonical CBOR-encoded payload map for that message type.
  * `signature_bstr` contains the signature bytes.
  * Signature input: from the first byte of the outer bstr content up to and including the end of `payload_bstr` (i.e., excludes the second array element entirely).
    This permits strict framing while signing the full payload.
  * Rationale: Wrapping the inner array as a bstr provides explicit length framing so receivers can bound input before decoding.

* Common payload fields use numeric map keys on the wire.
  Names map to numbers as follows:
  * 1: `ver` (uint) — protocol version (1)
  * 2: `uuid` (bstr .size 16) — UUIDv7 for deduplication/correlation
  * 3: `peer` (bstr) — sender peer-id bytes
  * 4: `ts` (uint) — sender-local milliseconds since Unix epoch
  * 5: `root` (bstr .size 32) — SMT root (BLAKE3-256; see SMT section)
  * 6: `count` (uint) — total document count after applying the operation
* Deduplication: Receivers MUST de-duplicate by `(peer, uuid)` and drop duplicates.
* Idempotence: Duplicated CIDs in `.new` are harmless; set inserts are idempotent.

CDDL — Common Types and Envelope

```cddl
; Common Message Envelope

; Envelope: outer bstr containing a signed CBOR payload
envelope = bstr .cbor signed-payload

signed-payload = [ 
    peer: peer-pubkey,
    seq: uuidv7,
    ver: uint,
    payload: payload-body, 
    signature_bstr: peer-sig 
]

uuidv7 = uuid
uuid = #6.37(bytes .size 16)

ed25519-pubkey = bytes .size 32
ed25519-sig = bytes .size 32

peer-pubkey = ed25519-pubkey
peer-sig = ed25519-sig

payload-body = { * uint => any }
```

Diagnostic example (envelope framing only):

```cbor
bstr([
  h'a1...payload...',  ; payload_bstr (CBOR bstr of the payload map)
  h'00...sig...'
])
```

## Message Types

### .new (topic `<base>.new`)

* Semantics: Announce newly produced documents and the sender’s resulting set summary.

* Payload (numeric keys; map inside `payload_bstr`):
  * Common fields: 1..6
  * 10: `batch` (array of `cid1`) OPTIONAL — inline new CIDs if total payload ≤ 1 MiB.
  * 11: `manifest` (`cid1`) OPTIONAL — CID of an IPFS object listing new CIDs when inline exceeds the limit.
* Processing:
  * Fetch and pin all CIDs from `batch` or `manifest` before insertion.
  * Atomic pinning: if any CID in the announcement cannot be fetched and pinned within the pinning retry window, the peer MUST NOT keep any partial pins from this announcement; it MUST release any partial pins and defer insertion.
  * Upon successful pin of all CIDs in the announcement, insert each CID into local SMT; compute local root.
  * If local root ≠ sender `root`, mark divergence w.r.t. `peer` and enter reconciliation backoff (see State Machines) unless parity is achieved during backoff via subsequent `.new`/`.dif`.

### .syn (topic `<base>.syn`)

* Semantics: Solicitation for reconciliation; includes requester’s sketch.

* Payload (numeric keys):
  * Common fields: 1..6 refer to the requester’s current state.
  * 9: `to` (peer-id) OPTIONAL — target peer-id.
  * 12: `iblt` (map) — requester’s sketch and parameters (see IBLT section).
* Processing:
  * Any peer MAY respond if it believes it can help reconcile; responders SHOULD use jitter (see Timers) and suppress if a suitable `.dif` appears.
  * Observers MAY use information to converge opportunistically, but `.syn` does not carry updates itself.

### .dif (topic `<base>.dif`)

* Semantics: Reconciliation reply; may carry a responder sketch, small raw CID lists, or a pointer to a diff manifest.

* Payload (numeric keys):
  * Common fields: 1..6 refer to the responder’s current state.
  * 7: `in_reply_to` (uuid) — UUIDv7 of the `.syn` being answered.
  * One or more of:
    * 12: `iblt` (map) OPTIONAL — responder sketch for bi-directional peeling.
    * 13: `missing_for_requester` (array of `cid1`) OPTIONAL — only if total payload ≤ 1 MiB.
    * 14: `diff_manifest` (`cid1`) OPTIONAL — CID of an IPFS object describing the diff (see Diff Manifest).
* Processing:
  * Requesters attempt to decode using provided sketches; if decoded, fetch+pin `missing_for_requester` (from inline list or manifest), update SMT, and check parity.
  * Observers MAY also use `.dif` to converge faster.

### .prv (topic `<base>.prv`, OPTIONAL)

* Semantics: Request SMT inclusion proof(s) for a specific CID from one or more peers.

* Payload (numeric keys):
  * Common fields: 1..6 are the requester’s current state.
  * 8: `cid` (`cid1`) — the document CID for which an inclusion proof is requested.
  * 15: `provers` (array of peer-id) OPTIONAL: explicit peers asked to respond.
    If omitted or empty, any peer MAY respond (subject to jitter).
  * 16: `hpke_pkR` (bstr .size 32) — requester’s ephemeral X25519 public key.
    REQUIRED.
* Processing:
  * If `provers` is present, only listed peers SHOULD answer; others SHOULD ignore to avoid unnecessary replies.
  * If `provers` is absent, any peer MAY volunteer a proof after responder jitter; responders DO NOT suppress based on other `.prf` replies (multiple independent proofs are acceptable).
  * `.prv` carries no updates by itself.

### .prf (topic `<base>.prf`, OPTIONAL)

* Semantics: Reply to a `.prv` with an SMT inclusion proof for the requested `cid`.

* Payload (numeric keys):
  * Common fields: 1..6 refer to the responder’s current state at proof time.
  * 7: `in_reply_to` (uuid) — UUIDv7 of the `.prv` being answered.
  * 8: `cid` (`cid1`) — the requested document CID.
  * 17: `hpke_enc` (bstr .size 32) — responder’s HPKE encapsulated ephemeral public key.
    REQUIRED.
  * 18: `ct` (bstr) — HPKE ciphertext of the proof payload (see Encrypted Proofs).
    REQUIRED.
* Processing:
  * Only the requester possessing the matching X25519 private key can decrypt `ct`.
  * After decryption, verify bindings and the SMT proof; see Encrypted Proofs.
  * Non-requesters cannot decrypt and SHOULD ignore the ciphertext.

## Proof Topics Usage Model (Optional)

* Roles:
  * Proven storage peers: nodes that commit to answering proof requests.
  * Non-proven peers: nodes that generally do not need proofs but may occasionally request them.
* Recommended subscription pattern:
  * Proven storage peers SHOULD remain subscribed to `<base>.prv` only.
    Upon receiving a `.prv` they intend to answer, they SHOULD temporarily subscribe to `<base>.prf`, apply responder jitter, publish their `.prf`, and promptly unsubscribe.
    They DO NOT suppress due to other `.prf` replies; proofs are tied to the responder’s storage commitment.
  * Non-proven peers SHOULD remain unsubscribed from proof topics under normal operation.
    When a proof is needed:
    1. Subscribe to `<base>.prv` and `<base>.prf`.
    2. Publish `.prv` specifying the `cid` (and optionally specific `provers`) and include `hpke_pkR` (ephemeral X25519 public key).
    3. Wait for `.prf` replies, decrypt, verify, and cache as needed.
    4. Unsubscribe from `<base>.prf` (and `<base>.prv` if no further requests).
* Rationale: This pattern effectively narrows `.prf` delivery to the requester and the responding prover(s) currently subscribed, approximating point-to-point behavior over pub/sub and reducing background load for nodes that do not need proofs.

## State Machines

* Local peer maintains per-remote-peer sync state for each `<base>`:
  * Stable: local root equals last known root for all known peers.
  * Diverged: a mismatch exists (local root ≠ any seen remote root).
    On entering Diverged, start backoff timer.
  * Reconciling: after backoff expiry, publish `.syn`; await suitable `.dif` and apply.
  * Parity achieved: upon local root matching the responder’s advertised root; return to Stable.

* Transitions may be triggered by `.new` or `.dif` arriving during backoff; if parity is reached, abort solicitation.

## Timers and Retries

* Backoff/jitter before sending `.syn`: uniform random in `[Tmin, Tmax]` (implementation-configurable; PoC suggestion: 200–800 ms).

* Responder jitter before publishing `.dif`: uniform random in `[Rmin, Rmax]` (PoC suggestion: 50–250 ms).
  Cancel if an adequate `.dif` appears.
* IBLT multi-round: if peeling fails, responder or requester MAY escalate parameters and send an additional `.dif` with a larger sketch; cap rounds to a small number (e.g., 2–3).
* Pinning retry window: implementations SHOULD configure a bounded retry window `Wpin` (e.g., tens of seconds) during which failed CID fetches from a single `.new` announcement are retried; if the window elapses without all CIDs pinned, release partial pins and schedule a later retry per node policy.
* Proof reply jitter: responders to `.prv` SHOULD wait a uniform random delay in `[Rmin, Rmax]` (same range as `.dif`) while temporarily subscribed to `<base>.prf`, then publish their `.prf`.

## Transport and Size Limits

* Pub/sub messages SHOULD be ≤ 1 MiB.
  Inline arrays of CIDs in `.new`/`.dif` MUST keep total message size ≤ 1 MiB.

* For larger batches or diffs, use manifests referenced by CID.

* Proof topics: `.prf` replies SHOULD respect the same ≤ 1 MiB bound.
  Large proofs (e.g., very deep sibling arrays) are unlikely due to SMT’s fixed size but MAY necessitate splitting across multiple `.prf` messages or providing a manifest CID if ever required.

## SMT (Sparse Merkle Tree)

* Purpose: Order-independent, append-only set with inclusion and non-inclusion proofs.

* Keying: For each CID, compute key `k = BLAKE3-256(CIDv1-bytes)`.
  Implementations MUST convert CIDs to their binary CIDv1 representation before hashing.
* Depth: 256 levels (one per bit of `k`).
* Hash function: BLAKE3-256 (fixed 32-byte output).
* Domain separation:
  * `LeafHash = BLAKE3-256(0x00 || k || 0x01)` (presence-only set; constant value 0x01).
  * `NodeHash = BLAKE3-256(0x01 || left || right)`.
  * `Empty[d]` precomputed per depth: `Empty[256] = BLAKE3-256(0x02)`, `Empty[d] = NodeHash(Empty[d+1], Empty[d+1])`.
* Root: 32-byte `NodeHash` at depth 0 (BLAKE3-256 output).
* Inclusion proof: path bits from `k` plus sibling hashes per level.
  Exclusion proof: proof of `Empty` at divergence depth or neighbor leaf.

## Encrypted Proofs (Mandatory for `.prf`)

* Algorithms (HPKE per RFC 9180 profile):
  * KEM: DHKEM(X25519, HKDF-SHA256)
  * KDF: HKDF-SHA256
  * AEAD: ChaCha20-Poly1305
* Request flow:
  * Requester generates an ephemeral X25519 key pair and publishes `.prv` including `hpke_pkR` (32-byte public key).
* Response flow:
  * Prover derives an HPKE context using `hpke_pkR`, generates `hpke_enc` (32-byte encapsulated pub), and encrypts the proof payload into `ct`.
* Ciphertext AAD binding:
  * The AEAD additional authenticated data MUST be the canonical CBOR encoding of the following map from the `.prf` outer payload: `{ver, peer, uuid, in_reply_to, cid, root, count}`.
  * This prevents transplanting `ct` under a different envelope.
* Encrypted plaintext format (canonical CBOR map with numeric keys; see `prf-plaintext` CDDL):
  * 1: `responder` (peer-id) — MUST equal outer `peer` (3)
  * 2: `in_reply_to` (uuid) — MUST match outer `in_reply_to` (7)
  * 3: `cid` (cid1) — MUST match outer `cid` (8)
  * 4: `root` (root32) — MUST match outer `root` (5)
  * 5: `count` (uint) — MUST match outer `count` (6)
  * 6: `present` (bool) — whether the CID is included
  * 7: `proof` (smt-proof) — inclusion/non-inclusion proof
* Verification (requester):
  1. Decapsulate with X25519 private key to obtain AEAD key/nonce, then decrypt `ct` using AAD as above.
  2. Check that `responder`, `in_reply_to`, `cid`, `root`, and `count` exactly match the outer payload fields.
  3. If `present = true`, verify the SMT inclusion proof against `root`; if `false`, verify the non-inclusion proof.
* Non-requesters cannot decrypt and SHOULD ignore `.prf` ciphertext.

### SMT Proof Encoding

* Proofs use numeric keys:
  * 1: `type` (uint) — 0 = inclusion, 1 = non-inclusion.
  * 2: `k` (bstr .size 32) — `k = BLAKE3-256(CIDv1-bytes)`.
  * 3: `siblings` (array of bstr .size 32) — ordered from leaf upward (LSB-first traversal).
  * 4: `leaf` (bstr .size 32) OPTIONAL — `LeafHash`; MAY be omitted.
  * 5: `depth` (uint) OPTIONAL — defaults to 256 if omitted.

CDDL — SMT proofs

```
smt-proof = {
  1 => uint,            ; 0 incl, 1 excl
  2 => bstr .size 32,   ; k
  3 => [* bstr .size 32],
  ? 4 => bstr .size 32, ; leaf
  ? 5 => uint           ; depth
}
```

* Verification procedure (inclusion):
  1. Compute `k` from the provided CID and compare to `proof.k`.
  2. Compute `LeafHash` using domain separation; iteratively hash with `siblings` per bit of `k` to reconstruct a candidate root.
  3. Accept if candidate root equals the responder’s `root` in the `.prf` payload.
* Verification procedure (non-inclusion):
  * Either demonstrate a path leading to an `Empty` node at divergence depth or provide a neighbor leaf proof whose key differs at the first differing bit.

## IBLT (Set Reconciliation)

* Objective: Identify set difference between requester and responder.

* Keys: `h = SHA-256(CIDv1-bytes)`; truncate to 64-bit key id for table operations; checksum = lower 32 bits of `SHA-256(0x03 || CIDv1-bytes)`.
* Parameters:
  * Hash count `k = 3`.
  * Initial table size `m`: `m = max(64, 3 * max(16, |count_responder - count_requester| + 8))`.
  * Escalation factor: multiply `m` by 1.6 per additional round, up to 2 rounds.
  * Seeds: derive k independent 64-bit seeds from `uuid` (HKDF-SHA256 with info = "hermes-iblt").
* Encoding (CBOR `iblt` map with numeric keys):
  * 1: `m` (uint), 2: `k` (uint), 3: `seeds` (array of k uint), 4: `cells` (array of `iblt-cell`).
  * `iblt-cell` = { 1: `c` (int), 2: `key_xor` (uint), 3: `chksum_xor` (uint) }.
* Requester includes its IBLT in `.syn`.
  Responder MAY include its own IBLT in `.dif` to enable bi-directional peeling.

CDDL — IBLT types

```
iblt = {
  1 => uint,              ; m
  2 => uint,              ; k
  3 => [* uint],          ; seeds (length k)
  4 => [* iblt-cell]
}
iblt-cell = {
  1 => int,               ; c
  2 => uint,              ; key_xor (fits in 64 bits)
  3 => uint               ; chksum_xor (fits in 32 bits)
}
```

## Diff Manifest (IPFS object)

* Use when inline lists would exceed 1 MiB.
* Numeric keys are used for fields:
  * 1: `ver` (uint)
  * 2: `in_reply_to` (uuid)
  * 3: `responder` (peer-id)
  * 4: `root` (root32)
  * 5: `count` (uint)
  * 6: `missing_for_requester` (array of `cid1`)
  * 7: `missing_for_responder` (array of `cid1`) OPTIONAL
  * 8: `iblt_params` (map) OPTIONAL
  * 9: `sig` (bstr) — signature by responder over the manifest body
* The `.dif` payload carries the manifest CID.

CDDL — Diff manifest

```
diff-manifest = {
  1 => ver,
  2 => uuid,
  3 => peer-id,     ; responder
  4 => root32,
  5 => uint,        ; count
  6 => [* cid1],
  ? 7 => [* cid1],
  ? 8 => { ? 1 => uint, ? 2 => uint },  ; iblt_params (m,k) if recorded
  9 => bstr
}
```

Diagnostic example (decoded):

```
{
  1: 1,
  2: h'018f0f92c3f8a9b2c7d1112233445567',
  3: h'44556677',
  4: h'cccc...cccc',
  5: 200,
  6: [ h'06f9...cid1', h'07aa...cid1' ],
  8: { 1: 256, 2: 3 },
  9: h'...sig...'
}
```

## Error Handling

* Invalid signature or non-canonical CBOR: drop.

* Oversized message: drop.
* Fetch/pin failure: do not insert into SMT; release partial pins from the announcement; retain a pending queue and retry within the configured pinning window/policy.
* IBLT peel failure: escalate once or twice; otherwise rely on manifest CID fallback.

## Security Considerations (PoC)

* Honest participants assumed; messages are public and unauthenticated beyond per-peer signatures, except `.prf` proof content which is encrypted end-to-end using HPKE as specified.

* Implementations SHOULD rate-limit `.syn` and `.dif` per peer and bound pin concurrency to avoid resource exhaustion.

## Privacy Considerations (PoC)

* None; all fields are public.
  Future versions may add encryption/signing via COSE.

## Observability and Metrics

* Track: `.new` seen, pins queued/succeeded/failed, roots observed, divergence detected, `.syn` sent, `.dif` received, IBLT peel success/failure, bytes fetched, manifests used.
* If proof topics enabled: `.prv` sent/received, `.prf` verified, proof cache hits.

## Interoperability

* Canonical CBOR is required for all payloads and manifests.
  CIDs MUST be CIDv1; CID arrays must contain canonical representations.
* SMT hashing MUST be BLAKE3-256 exactly as specified; mixing hash functions will produce incompatible roots and proofs.

## Extensibility

* New optional fields may be added to payload maps.
  Unknown optional fields MUST be ignored.

## Conformance and Test Vectors

* Provide fixtures for `.new`, `.syn`, `.dif`, a small SMT set, and IBLT peeling cases (TBD in repository).

## References

* IBLT: Goodrich & Mitzenmacher (2011), "Invertible Bloom Lookup Tables".

* Sparse Merkle Trees: RFC 6962 (conceptual), Cosmos ICS23 (proof encoding inspiration).
* BLAKE3: O'Connor et al., <https://github.com/BLAKE3-team/BLAKE3> (specification and reference implementations).

## Open Questions

* Numeric defaults (`Tmin/Tmax`, size caps) may be tuned through experimentation.

* Potential future direct-stream optimization for large diffs.
CDDL — `.new` payload

```
new-payload = common // {
  ? 10 => [* cid1],   ; batch
  ? 11 => cid1        ; manifest
}
```

Diagnostic example (payload_bstr decoded):

```
{
  1: 1,                                        ; ver
  2: h'018f0f92c3f8a9b2c7d1112233445566',      ; uuid
  3: h'aabbccdd',                              ; peer
  4: 1710000000000,                            ; ts
  5: h'0123456789ab...0123',                   ; root (BLAKE3-256)
  6: 42,                                       ; count
  10: [ h'01a4...cid1', h'02b5...cid1' ]       ; batch (CIDv1 binary)
}
```

CDDL — `.syn` payload

```
syn-payload = common // {
  ? 9  => peer-id,  ; to
  12 => iblt        ; requester sketch
}
```

Diagnostic example (payload_bstr decoded):

```
{
  1: 1,
  2: h'018f0f92c3f8a9b2c7d1112233445567',
  3: h'deafbeef',
  4: 1710000000100,
  5: h'aaaa...aaaa',
  6: 100,
  9: h'cafebabe',
  12: { 1: 128, 2: 3, 3: [123456, 789012, 345678], 4: [ {1:0,2:0,3:0} ] }
}
```

CDDL — `.dif` payload

```
dif-payload = common // {
  7  => uuid,
  ? 12 => iblt,
  ? 13 => [* cid1],
  ? 14 => cid1
}
```

Diagnostic example (payload_bstr decoded, inline missing list):

```
{
  1: 1,
  2: h'018f0f92c3f8a9b2c7d1112233445568',
  3: h'00112233',
  4: 1710000000200,
  5: h'bbbb...bbbb',
  6: 105,
  7: h'018f0f92c3f8a9b2c7d1112233445567',
  13: [ h'03c6...cid1', h'04d7...cid1' ]
}
```

CDDL — `.prv` payload

```
prv-payload = common // {
  8  => cid1,
  ? 15 => [* peer-id],
  16 => bstr .size 32   ; hpke_pkR
}
```

Diagnostic example (payload_bstr decoded):

```
{
  1: 1,
  2: h'018f0f92c3f8a9b2c7d1112233445570',
  3: h'99887766',
  4: 1710000000400,
  5: h'dddd...dddd',
  6: 200,
  8: h'05e8...cid1',
  15: [ h'aa11bb22', h'cc33dd44' ],
  16: h'5566...'
}
```

CDDL — `.prf` payload and encrypted plaintext

```
prf-payload = common // {
  7  => uuid,
  8  => cid1,
  17 => bstr .size 32,  ; hpke_enc
  18 => bstr            ; ct
}

; Encrypted plaintext structure inside ct
prf-plaintext = {
  1 => peer-id,      ; responder (must equal outer 3)
  2 => uuid,         ; in_reply_to (must equal outer 7)
  3 => cid1,         ; cid (must equal outer 8)
  4 => root32,       ; root (must equal outer 5)
  5 => uint,         ; count (must equal outer 6)
  6 => bool,         ; present
  7 => smt-proof     ; proof (incl/excl)
}
```

Diagnostic example (outer payload_bstr decoded):

```
{
  1: 1,
  2: h'018f0f92c3f8a9b2c7d1112233445571',
  3: h'aa11bb22',
  4: 1710000000500,
  5: h'dddd...dddd',
  6: 201,
  7: h'018f0f92c3f8a9b2c7d1112233445570',
  8: h'05e8...cid1',
  17: h'1122...',
  18: h'99aa...'
}
```

Diagnostic example (decrypted prf-plaintext):

```
{
  1: h'aa11bb22',                         ; responder
  2: h'018f0f92c3f8a9b2c7d1112233445570', ; in_reply_to
  3: h'05e8...cid1',                       ; cid
  4: h'dddd...dddd',                       ; root
  5: 201,                                   ; count
  6: true,                                  ; present
  7: { 1: 0, 2: h'1f2e...00', 3: [ h'ab..1', h'ab..2' ] } ; smt-proof
}
```
