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
* Proof topics are OPTIONAL. Topics that require verifiability SHOULD additionally subscribe to `<base>.prv` and `<base>.prf`.

## Message Model

* Framing and Signature Envelope:
  * Each published message item is a CBOR byte string (bstr), whose content is a canonical CBOR array of two elements: `[payload_bstr, signature_bstr]`.
  * `payload_bstr` is a CBOR byte string containing the canonical CBOR-encoded payload map for that message type.
  * `signature_bstr` contains the signature bytes.
  * Signature input: from the first byte of the outer bstr content up to and including the end of `payload_bstr` (i.e., excludes the second array element entirely).
    This permits strict framing while signing the full payload.
  * Rationale: Wrapping the inner array as a bstr provides explicit length framing so receivers can bound input before decoding.

* Common payload fields (present in all message types unless noted):
  * `ver` (uint): protocol version (1).
  * `uuid` (bstr, 16 bytes): UUIDv7 for deduplication and correlation.
  * `peer` (bstr): sender peer-id bytes.
  * `ts` (uint): sender-local milliseconds since Unix epoch per UUIDv7 time or local clock.
  * `root` (bstr): SMT root (32 bytes for SHA-256; see SMT section).
  * `count` (uint): total document count in the sender’s set after applying the operation described by the message.
* Deduplication: Receivers MUST de-duplicate by `(peer, uuid)` and drop duplicates.
* Idempotence: Duplicated CIDs in `.new` are harmless; set inserts are idempotent.

## Message Types

### .new (topic `<base>.new`)

* Semantics: Announce newly produced documents and the sender’s resulting set summary.

* Payload (map inside `payload_bstr`):
  * Common fields.
  * `batch` (array of CID items) OPTIONAL: inline new CIDs if total payload ≤ 1 MiB.
  * `manifest` (CID) OPTIONAL: CID of an IPFS object listing the new CIDs when the inline batch would exceed 1 MiB.
* Processing:
  * Fetch and pin all CIDs from `batch` or `manifest` before insertion.
  * Atomic pinning: if any CID in the announcement cannot be fetched and pinned within the pinning retry window, the peer MUST NOT keep any partial pins from this announcement; it MUST release any partial pins and defer insertion.
  * Upon successful pin of all CIDs in the announcement, insert each CID into local SMT; compute local root.
  * If local root ≠ sender `root`, mark divergence w.r.t. `peer` and enter reconciliation backoff (see State Machines) unless parity is achieved during backoff via subsequent `.new`/`.dif`.

### .syn (topic `<base>.syn`)

* Semantics: Solicitation for reconciliation; includes requester’s sketch.

* Payload:
  * Common fields, where `root` and `count` refer to the requester’s current state.
  * `to` (bstr) OPTIONAL: target peer-id.
    If present, responders other than `to` SHOULD suppress response unless no reply observed after jitter.
  * `iblt` (map): requester’s sketch and parameters (see IBLT section).
* Processing:
  * Any peer MAY respond if it believes it can help reconcile; responders SHOULD use jitter (see Timers) and suppress if a suitable `.dif` appears.
  * Observers MAY use information to converge opportunistically, but `.syn` does not carry updates itself.

### .dif (topic `<base>.dif`)

* Semantics: Reconciliation reply; may carry a responder sketch, small raw CID lists, or a pointer to a diff manifest.

* Payload:
  * Common fields, where `root` and `count` refer to the responder’s current state.
  * `in_reply_to` (bstr, 16): UUIDv7 of the `.syn` being answered.
  * One or more of:
    * `iblt` (map) OPTIONAL: responder sketch for bi-directional peeling.
    * `missing_for_requester` (array of CIDs) OPTIONAL: only if total payload ≤ 1 MiB.
    * `diff_manifest` (CID) OPTIONAL: CID of an IPFS object describing the diff (see Diff Manifest).
* Processing:
  * Requesters attempt to decode using provided sketches; if decoded, fetch+pin `missing_for_requester` (from inline list or manifest), update SMT, and check parity.
  * Observers MAY also use `.dif` to converge faster.

### .prv (topic `<base>.prv`, OPTIONAL)

* Semantics: Request SMT inclusion proof(s) for a specific CID from one or more peers.

* Payload:
  * Common fields, where `root` and `count` are the requester’s current state.
  * `cid` (CID): the document CID for which an inclusion proof is requested.
  * `provers` (array of bstr peer-ids) OPTIONAL: explicit peers asked to respond. If omitted or empty, any peer MAY respond (subject to jitter).
  * `hpke_pkR` (bstr, 32): requester’s ephemeral X25519 public key for encrypted proof replies. This field is REQUIRED.
* Processing:
  * If `provers` is present, only listed peers SHOULD answer; others SHOULD ignore to avoid unnecessary replies.
  * If `provers` is absent, any peer MAY volunteer a proof after responder jitter; responders DO NOT suppress based on other `.prf` replies (multiple independent proofs are acceptable).
  * `.prv` carries no updates by itself.

### .prf (topic `<base>.prf`, OPTIONAL)

* Semantics: Reply to a `.prv` with an SMT inclusion proof for the requested `cid`.

* Payload:
  * Common fields, where `root` and `count` refer to the responder’s current state at proof time.
  * `in_reply_to` (bstr, 16): UUIDv7 of the `.prv` being answered.
  * `cid` (CID): the requested document CID.
  * `hpke_enc` (bstr, 32): responder’s HPKE encapsulated ephemeral public key. This field is REQUIRED.
  * `ct` (bstr): HPKE ciphertext of the proof payload (see Encrypted Proofs). This field is REQUIRED.
* Processing:
  * Only the requester possessing the matching X25519 private key can decrypt `ct`.
  * After decryption, verify bindings and the SMT proof; see Encrypted Proofs.
  * Non-requesters cannot decrypt and SHOULD ignore the ciphertext.

## Proof Topics Usage Model (Optional)

* Roles:
  * Proven storage peers: nodes that commit to answering proof requests.
  * Non-proven peers: nodes that generally do not need proofs but may occasionally request them.
* Recommended subscription pattern:
  * Proven storage peers SHOULD remain subscribed to `<base>.prv` only. Upon receiving a `.prv` they intend to answer, they SHOULD temporarily subscribe to `<base>.prf`, apply responder jitter, publish their `.prf`, and promptly unsubscribe. They DO NOT suppress due to other `.prf` replies; proofs are tied to the responder’s storage commitment.
  * Non-proven peers SHOULD remain unsubscribed from proof topics under normal operation. When a proof is needed:
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

* Proof topics: `.prf` replies SHOULD respect the same ≤ 1 MiB bound. Large proofs (e.g., very deep sibling arrays) are unlikely due to SMT’s fixed size but MAY necessitate splitting across multiple `.prf` messages or providing a manifest CID if ever required.

## SMT (Sparse Merkle Tree)

* Purpose: Order-independent, append-only set with inclusion and non-inclusion proofs.

* Keying: For each CID, compute key `k = SHA-256(CIDv1-bytes)`.
  Implementations MUST convert CIDs to their binary CIDv1 representation before hashing.
* Depth: 256 levels (one per bit of `k`).
* Hash function: SHA-256.
* Domain separation:
  * `LeafHash = SHA-256(0x00 || k || 0x01)` (presence-only set; constant value 0x01).
  * `NodeHash = SHA-256(0x01 || left || right)`.
  * `Empty[d]` precomputed per depth: `Empty[256] = SHA-256(0x02)`, `Empty[d] = NodeHash(Empty[d+1], Empty[d+1])`.
* Root: 32-byte `NodeHash` at depth 0.
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
* Encrypted plaintext format (canonical CBOR map):
  * `responder` (bstr): peer-id of the prover (MUST equal outer `peer`).
  * `in_reply_to` (bstr,16): MUST match the outer `in_reply_to`.
  * `cid` (CID): MUST match the outer `cid`.
  * `root` (bstr): responder’s root at proof time (MUST match outer `root`).
  * `count` (uint): responder’s count at proof time (MUST match outer `count`).
  * `present` (bool): whether the CID is included.
  * `proof` (map): SMT proof object (see SMT Proof Encoding). For non-inclusion, `type = "excl"`.
* Verification (requester):
  1. Decapsulate with X25519 private key to obtain AEAD key/nonce, then decrypt `ct` using AAD as above.
  2. Check that `responder`, `in_reply_to`, `cid`, `root`, and `count` exactly match the outer payload fields.
  3. If `present = true`, verify the SMT inclusion proof against `root`; if `false`, verify the non-inclusion proof.
* Non-requesters cannot decrypt and SHOULD ignore `.prf` ciphertext.

### SMT Proof Encoding

* Proofs are canonical CBOR maps with the following fields:
  * `type` (tstr): "incl" for inclusion proofs; "excl" for non-inclusion proofs.
  * `k` (bstr, 32): the key `k = SHA-256(CIDv1-bytes)`.
  * `siblings` (array of bstr): ordered array of 32-byte sibling hashes from leaf level up to root (least-significant bit first traversal).
  * `leaf` (bstr, 32) OPTIONAL: the computed `LeafHash` for inclusion proofs; MAY be omitted (verifier can recompute from `k`).
  * `depth` (uint) OPTIONAL: total tree depth (defaults to 256 if omitted).
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
* Encoding (CBOR `iblt` map):
  * `m` (uint), `k` (uint), `seeds` (array of k uint64), `cells` (array of cells), where each cell = `{c: int, key_xor: uint64, chksum_xor: uint32}`.
* Requester includes its IBLT in `.syn`.
  Responder MAY include its own IBLT in `.dif` to enable bi-directional peeling.

## Examples

The following examples use CBOR diagnostic notation for readability; actual payloads must be canonical CBOR and wrapped per the envelope rules.

### Example: `.new` with small inline batch

Envelope (outer):

  bstr(
    [
      payload_bstr,
      signature_bstr
    ]
  )

payload_bstr (decoded map):

  {
    ver: 1,
    uuid: h'018f0f92c3f8a9b2c7d1112233445566',
    peer: h'aabbccdd',
    ts: 1710000000000,
    root: h'012345...89ab',
    count: 42,
    batch: [
      cid("bafybeigdyrzt..."),
      cid("bafybeia6om3z...")
    ]
  }

signature_bstr: h'...'

### Example: `.syn` with IBLT

payload_bstr (decoded map):

  {
    ver: 1,
    uuid: h'018f0f92c3f8a9b2c7d1112233445567',
    peer: h'deafbeef',
    ts: 1710000000100,
    root: h'aaaaaa...aaaa',
    count: 100,
    to: h'cafebabe',
    iblt: {
      m: 128,
      k: 3,
      seeds: [ 123456, 789012, 345678 ],
      cells: [ {c:0, key_xor:0, chksum_xor:0}, {c:1, key_xor: 0x1122, chksum_xor: 0x3344} ]
    }
  }

### Example: `.dif` with small inline missing list

payload_bstr (decoded map):

  {
    ver: 1,
    uuid: h'018f0f92c3f8a9b2c7d1112233445568',
    peer: h'00112233',
    ts: 1710000000200,
    root: h'bbbbbb...bbbb',
    count: 105,
    in_reply_to: h'018f0f92c3f8a9b2c7d1112233445567',
    missing_for_requester: [
      cid("bafybeif7w3u..."),
      cid("bafybeih4k2j...")
    ]
  }

### Example: `.dif` with diff manifest

payload_bstr (decoded map):

  {
    ver: 1,
    uuid: h'018f0f92c3f8a9b2c7d1112233445569',
    peer: h'44556677',
    ts: 1710000000300,
    root: h'cccccc...cccc',
    count: 200,
    in_reply_to: h'018f0f92c3f8a9b2c7d1112233445567',
    diff_manifest: cid("bafybeigd3ffm...")
  }

Diff manifest (decoded map at `bafybeigd3ffm...`):

  {
    ver: 1,
    in_reply_to: h'018f0f92c3f8a9b2c7d1112233445567',
    responder: h'44556677',
    root: h'cccccc...cccc',
    count: 200,
    missing_for_requester: [ cid("bafy...1"), cid("bafy...2") ],
    iblt_params: { m: 256, k: 3 },
    sig: h'...'
  }

### Example: `.prv` request

payload_bstr (decoded map):

  {
    ver: 1,
    uuid: h'018f0f92c3f8a9b2c7d1112233445570',
    peer: h'99887766',
    ts: 1710000000400,
    root: h'dddddd...dddd',
    count: 200,
    cid: cid("bafybeif7w3u..."),
    provers: [ h'aa11bb22', h'cc33dd44' ],
    hpke_pkR: h'5566...'
  }

### Example: `.prf` reply with encrypted inclusion proof

payload_bstr (decoded map):

  {
    ver: 1,
    uuid: h'018f0f92c3f8a9b2c7d1112233445571',
    peer: h'aa11bb22',
    ts: 1710000000500,
    root: h'dddddd...dddd',
    count: 201,
    in_reply_to: h'018f0f92c3f8a9b2c7d1112233445570',
    cid: cid("bafybeif7w3u..."),
    hpke_enc: h'1122...',
    ct: h'99aa...'
  }

Decrypted plaintext (decoded map):

  {
    responder: h'aa11bb22',
    in_reply_to: h'018f0f92c3f8a9b2c7d1112233445570',
    cid: cid("bafybeif7w3u..."),
    root: h'dddddd...dddd',
    count: 201,
    present: true,
    proof: {
      type: "incl",
      k: h'1f2e3d...00',
      siblings: [ h'abc...1', h'abc...2', h'abc...3' ]
    }
  }

## Diff Manifest (IPFS object)

* Use when inline lists would exceed 1 MiB.

* Canonical CBOR map with fields:
  * `ver` (1), `in_reply_to` (uuid), `responder` (peer-id), `root` (responder root), `count` (responder count),
  * `missing_for_requester` (array of CIDs), OPTIONAL `missing_for_responder` (array of CIDs),
  * `iblt_params` (map) OPTIONAL recording parameters used,
  * `sig` (bstr) signature by responder over the manifest body.
* The `.dif` payload carries the manifest CID.

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

## Extensibility

* New optional fields may be added to payload maps.
  Unknown optional fields MUST be ignored.

## Conformance and Test Vectors

* Provide fixtures for `.new`, `.syn`, `.dif`, a small SMT set, and IBLT peeling cases (TBD in repository).

## References

* IBLT: Goodrich & Mitzenmacher (2011), "Invertible Bloom Lookup Tables".

* Sparse Merkle Trees: RFC 6962 (conceptual), Cosmos ICS23 (proof encoding inspiration).

## Open Questions

* Numeric defaults (`Tmin/Tmax`, size caps) may be tuned through experimentation.

* Potential future direct-stream optimization for large diffs.
