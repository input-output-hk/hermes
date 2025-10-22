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
  Three pub/sub topics per set: `<base>.new`, `<base>.syn`, `<base>.dif`.
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
  * `<base>.dif` (reconciliation replies and/or pointers to diff manifests).

* No direct streams are required in this PoC; all reconciliation occurs on pub/sub.

## Topics and Namespacing

* `<base>` is an opaque UTF-8 string under 120 characters, defined by higher-level context.

* Topic semantics are single-purpose: a topic MUST carry only its designated message type.
  Peers MAY drop senders violating this.

## Message Model

* Framing and Signature Envelope:
  * Each published message item is a CBOR byte string (bstr), whose content is a canonical CBOR array of two elements: `[payload_bstr, signature_bstr]`.
  * `payload_bstr` is a CBOR byte string containing the canonical CBOR-encoded payload map for that message type.
  * `signature_bstr` contains the signature bytes.
  * Signature input: from the first byte of the outer bstr content up to and including the end of `payload_bstr` (i.e., excludes the second array element entirely).
    This permits strict framing while signing the full payload.

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
  * Upon successful pin, insert each CID into local SMT; compute local root.
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

## Transport and Size Limits

* Pub/sub messages SHOULD be ≤ 1 MiB.
  Inline arrays of CIDs in `.new`/`.dif` MUST keep total message size ≤ 1 MiB.

* For larger batches or diffs, use manifests referenced by CID.

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
* Fetch/pin failure: do not insert into SMT; retain pending queue and retry per node policy.
* IBLT peel failure: escalate once or twice; otherwise rely on manifest CID fallback.

## Security Considerations (PoC)

* Honest participants assumed; messages are public and unauthenticated beyond per-peer signatures.

* Implementations SHOULD rate-limit `.syn` and `.dif` per peer and bound pin concurrency to avoid resource exhaustion.

## Privacy Considerations (PoC)

* None; all fields are public.
  Future versions may add encryption/signing via COSE.

## Observability and Metrics

* Track: `.new` seen, pins queued/succeeded/failed, roots observed, divergence detected, `.syn` sent, `.dif` received, IBLT peel success/failure, bytes fetched, manifests used.

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
