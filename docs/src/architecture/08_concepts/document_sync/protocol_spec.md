---
icon: material/file-document-outline
---

# IPFS Document Sync Protocol — Specification (Draft)

## Status

* Stage: PoC draft

* Version: 0.1.0 (wire format frozen per this document; numeric parameters may be tuned)

## Overview

**Purpose:** Efficiently synchronize a set of document CIDs across peers using
  IPFS pub/sub broadcast for announcements and manifest-based set reconciliation for divergence.

**Design:** Append-only document set represented by a Sparse Merkle Tree (SMT) root.
  Three required pub/sub topics per set: `<base>.new`, `<base>.syn`, `<base>.dif`.
  Two optional topics for proofs: `<base>.prv` (proof requests) and `<base>.prf` (proof replies).

**Assumptions (PoC):** Honest peers, no privacy, all messages publicly readable.
  All payloads use deterministic CBOR encoding with strict framing.

### Diagram — High-level Sync Flow

```mermaid
sequenceDiagram
  autonumber
  participant A as Peer A
  participant D as DHT
  participant PS as IPFS PubSub
  participant B as Peer B

  A-->>PS: subscribed to<br>`<base>.new` + `<base>.syn`
  B-->>PS: subscribed to<br>`<base>.new` + `<base>.syn`

  rect rgb(245,245,245)
    Note over A: Pre-publish check for .new
    A->>D: Provide(docs/manifest CIDs)
    A->>D: FindProviders(CID)
    D-->>A: provider ≠ A (OK)
  end
  Note over A: Broadcast on<br><base>.new
  A->>PS: .new [docs/manifest, root, count]
  PS-->>B: .new
  B->>B: fetch+pin, update SMT (root_B)
  
  alt roots differ
    Note over B: Broadcast on<br>`<base>.syn`
    B-->>PS: subscribe to `<base>.dif` during reconciling
    B->>PS: `<base>.syn`<br>[peer A, root_B, count_B]
    PS-->>A: .syn
    A-->>PS: subscribed to `<base>.dif` during reconciling
    rect rgb(245,245,245)
      Note over A: Pre-publish check for .dif
      A->>D: Provide(docs/manifest CIDs)
      A->>D: FindProviders(CID)
      D-->>A: provider ≠ A (OK)
    end
    A->>PS: .dif [in_reply_to, (docs | manifest), root_A, count_A, ttl?]
    A-->>PS: unsubscribe from `<base>.dif`
    PS-->>B: .dif
    B->>B: fetch+pin, update SMT → parity
    B-->>PS: unsubscribe from `<base>.dif`
  else already at parity
    B->>B: no sync needed
  end
```

## Scope and Goals

* Ensure eventual consistency of document sets across honest peers.
* Minimize pub/sub bandwidth via batched announcements and manifest-based diffs.
* Idempotent processing of duplicates and replays.
* Generic over `<base>` topic names; multiple sets may run in parallel.

## Non-Goals

* Adversarial hardening (Sybil/DoS resistance, spam prevention).
* Confidentiality or payload encryption.
* Membership/admission control.

## Roles and Entities

* **Peer:** A participant with an IPFS/libp2p node publishing and subscribing to the three topics for a given `<base>`.

* **Set:** The document set scoped to `<base>`.

## Protocol Versioning and Negotiation

* Field `ver` within payloads carries protocol version (uint).
  This document defines `ver = 1`.

* Implementations MUST ignore unknown optional fields; unknown critical fields MUST cause rejection.

## Transport Bindings

**Pub/Sub:** libp2p gossipsub (via IPFS pubsub).
All messages are broadcasts on the following topic designations:

* `<base>.new` : announcements of new CIDs and the sender’s resulting root
* `<base>.syn` : synchronization solicitations for reconciliation
* `<base>.dif` : reconciliation replies with pointers to diff manifests or small inline lists
* `<base>.prv` : *OPTIONAL* requests for SMT inclusion proofs of specific CIDs
* `<base>.prf` : *OPTIONAL* proof replies containing SMT proofs

No direct streams are required in this PoC; all reconciliation occurs on pub/sub.
The broadcast nature of Pub/Sub is utilized to improve efficiency in the sync process by
allowing all peers to observe the sync processes and preemptively update.

### How multiple topics help the system scale

**Processing isolation:**

* `.new` is small and frequent (control-plane for appends),
* `.syn` is synchronization request control-only,
* `.dif` can be large or bursty, and
* `.prv/.prf` are occasional and ephemeral.
  Separating channels prevents heavy `.dif` traffic from delaying `.new` processing and keeps control paths fast.

**Gossipsub performance:** IPFS/libp2p maintains meshes, validation, and scoring per-topic.
  Splitting topics allows independent backpressure, scoring, and message ID de-dup caches,
  improving fairness and convergence under load.

**Selective subscription:** Peers can opt out of proof traffic entirely,
  or temporarily subscribe to `.prf` only when they request a proof,
  reducing baseline bandwidth and CPU.

**Policy and rate limits:** Enforce “one message kind per topic” and apply separate quotas/priorities.
  For example, prioritize `.new` over `.dif` so new content signals propagate quickly, while large diffs trickle.

**Observability:** Topic-level metrics and alarms make it easy to detect misconfiguration or abuse
  (e.g., oversized `.dif` bursts) and to tune thresholds per traffic class.

**Future extensions:** Per-topic admission or encryption can be added without impacting unrelated flows.

#### Diagram — Topic Separation and QoS

```mermaid
flowchart LR
  subgraph PubSub Topics
    NEW["`&ltbase>.new<br>*small, frequent*`"]
    SYN["`&ltbase>.syn<br>*control-only*`"]
    DIF["`&ltbase>.dif<br>*large/bursty, ephemeral*`"]
    PRV["`&ltbase>.prv<br>*optional*`"]
    PRF["`&ltbase>.prf<br>*optional, ephemeral*`"]
  end

  NEW -- high priority --> MeshNew[per-topic mesh + scoring]
  SYN -- high priority --> MeshSyn
  DIF -.ephemeral+rate-limit .-> MeshDif
  PRV -.opt-in.-> MeshPrv
  PRF -.ephemeral.-> MeshPrf

  MeshNew --> Peers
  MeshSyn --> Peers
  MeshDif --> Peers
  MeshPrv --> Peers
  MeshPrf --> Peers
```

## Topics and Namespacing

`<base>` is an opaque UTF-8 string under 120 characters, defined by higher-level context.

Topic semantics are single-purpose: a topic MUST carry only its designated message type.
Peers MAY drop senders violating this.

*Proof topics are OPTIONAL.*
Topics that require verifiability SHOULD additionally subscribe to `<base>.prv` and `<base>.prf`.

### Sync Topic Subscription

* Peers SHOULD always subscribe to `<base>.new` and `<base>.syn`.
* Peers SHOULD subscribe to `<base>.dif` only while reconciling (Diverged/Reconciling states) and
  SHOULD unsubscribe when Stable to reduce baseline traffic.

#### Diagram — Subscribe/Unsubscribe During Sync

```mermaid
sequenceDiagram
  participant P as Peer
  participant PS as PubSub
  
  rect rgb(245,245,245)
    Note over P: Stable
    P->>PS: sub(`<base>.new` + `<base>.syn`)
    P-x PS: (no sub to `<base>.dif`)
  end

  Note over P: Diverged detected
  P->>PS: sub(`<base>.dif`)
  PS-->>P: .dif (responses)
  P->>PS: After Backoff Timeout:<br>pub `<base>.syn`
  PS-->>P: .dif (responses)

  Note over P: Parity achieved
  P->>PS: unsub(`<base>.dif`)
  P->>PS: keep(`<base>.new`) and (`<base>.syn`)
```

## Message Model
<!-- markdownlint-disable MD033 -->
Framing and Signature Envelope (matches the common envelope CDDL provided):

* Each message is a CBOR byte string (bstr) whose content is a CBOR array encoded deterministically:<br>
  `signed-payload = [ peer, seq, ver, payload, signature_bstr]`.
* Signature input: computed over the deterministic CBOR encoding of the bstr wrapper and first four elements
  `[peer-pubkey, seq, ver, payload-body]`
  (i.e., from the first byte of the envelope content up to the byte before `signature_bstr`).
<!-- markdownlint-enable MD033 -->

**Rationale:** Outer bstr provides explicit length framing; deterministic CBOR ensures unambiguous signing.

**Deduplication:** Receivers MUST de-duplicate by `(peer-pubkey, seq)` and drop duplicates.

**Idempotence:** Duplicated CIDs in `.new` are harmless; set inserts are idempotent.

### Envelope Field Definitions

1. **peer** - The Ed25519 Public Key which matches the senders Peer ID in the IPFS Network.
   The senders peer ID can be derived from this public key, and must be valid.
2. **seq** - Message Sequence.
   A UUIDv7 that defined both a unique Nonce, and a Timestamp.
   Prevents and helps detect message duplication.
3. **ver** - Numeric protocol version.
   This protocol is Version 1, so this will be encoded as `1`.
4. **payload** - The channel specific payload contained within the message.
5. **signature** - The signature over all preceding bytes, from the bstr header to the end of the payload.
   The signature is made using the senders private ED25519 key which matches their public key.
   It is validated against the **peer** field at the start of the payload.

### Diagram — Envelope and Payload

```mermaid
---
config:
  layout: elk
---

erDiagram
  Envelope {
    bstr(82-1MiB) SignedPayload
  }

  SignedPayload {
    ed25519-pubkey(32) peer
    uuidv7(16) seq
    uint ver
    payload-body(map) payload
    ed25519-sig(64) signature
  }

  Envelope ||--|| SignedPayload : Contains

```

### Common Message Envelope

```cddl
; Common Message Envelope

; Envelope: outer bstr containing a signed CBOR payload
envelope = bstr .size (82..1048576) .cbor signed-payload

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

### Diagnostic example (envelope framing only)

```cbor
bstr([
  h'aa..peer-pubkey..',   ; peer (ed25519 pubkey)
  h'01..uuidv7..',        ; seq
  1,                      ; ver = 1
  { 1: h'root..', 2: 42 },; payload-body (example for .new)
  h'..signature..'        ; peer-sig
])
```

## Message Types

### Document Dissemination (shared payload for `.new` and `.dif`)

This generic message describes the common payload-body used by both `.new` and `.dif`.

Topic-specific rules (e.g., `.dif` requiring `in_reply_to`, `.new` forbidding it) are defined in their respective sections below.

**Semantics:** Announce documents and the sender’s resulting set summary.

Payload-body Keys:

| Index | Name | Type | Description |
| --- | --- | --- | --- |
| 1 | **root** | root-hash | BLAKE3-256 of the sender’s SMT root |
| 2 | **count** | uint | sender’s current document count |
| 3 | **docs**  | array of cidv1<br>*OPTIONAL* | inline CIDs the sender believes others may be missing (≤ 1 MiB total) |
| 4 | **manifest** | cidv1<br>*OPTIONAL* | manifest CID listing CIDs when message size would exceed 1 MiB. |
| 5 | **ttl** | uint<br>*OPTIONAL* | seconds the manifest remains available.<br>The responder will keep the manifest block available for this time.<br>Time starts at the time represented by the envelope’s UUIDv7 (seq).<br>*[default 3600 (1 Hour) if not present]*. |
| 6 | **in_reply_to** | UUIDv7 | `seq` of the `.syn` message which caused this message to be sent. (Not used in `.new`) |

Either **docs** or **manifest** Must be present, and only one of them may be present.

#### Document dissemination payload-body

```cddl

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
```

**Note:** *Only CIDv1 with multihash sha2-256 is permitted for document CIDs in this PoC;
implementations MUST reject other multihash functions.*

##### CIDv1 binary encoding (PoC focus)

Layout (packed bytes): `cidv1 = varint(1) || varint(multicodec) || multihash`.

Multihash for this PoC MUST be sha2-256 with a 32-byte digest:

`multihash = varint(0x12) || varint(32) || digest[32]`.

Length varies slightly (36..40 bytes) due to varint encoding of the multicodec field.

Other multihash functions are Invalid.

###### References

* CIDv1 specification: <https://github.com/multiformats/cid>
* Multicodec table: <https://github.com/multiformats/multicodec>
* Multihash specification: <https://github.com/multiformats/multihash>

##### Pre publication document availability check

Pre-publication availability check **(MUST)** be performed before announcing or referencing a document CID on the network
(i.e., publishing a payload on `.new`, `.dif`, or `.prv` that contains a `cidv1` either inline or via a manifest),
the publishing peer MUST ensure the CID is discoverable via the DHT by:

1. Providing the CID (announce provider records) to the DHT, and
2. Successfully calling FindProviders(CID) and obtaining at least one provider peer ID that is NOT the publisher’s own peer ID.

If this check fails, the publisher MUST retry Provide/FindProviders with backoff before sending the announcement.
For manifests, this requirement applies to the manifest CID itself and to every CID listed within the manifest.

Rationale: This ensures receiving peers can discover and fetch/pin the referenced content promptly after observing the announcement.

##### Diagram — Choosing docs vs manifest

```mermaid
flowchart TD
  A[Start] --> C{Estimated encoded size ≤ 1 MiB?}
  C -- Yes --> D[Include docs inline]
  C -- No --> M["Publish manifest (cidv1)"]
  M --> T["Set ttl (seconds)"]
  D --> P1["Provide(CID); FindProviders(CID) != self"]
  T --> P2["Provide(manifest CID & docs); FindProviders != self"]
  P1 --> E[Send payload]
  P2 --> E
```

#### Diagnostic example (payload-body decoded)

```cbor
{ 
    1: h'aaaa...aaaa', 
    2: 100, 
    3: [ 
        h'01..cid1', 
        h'02..cid1' 
    ] 
}
```

### `.new` (topic `<base>.new`)

**Semantics:** Announce newly produced documents and proactively disseminate documents the sender believes others may be missing.

**Payload-body:** Uses the shared Document Dissemination payload (see “Document Dissemination”).

**Topic rules:**

* *in_reply_to* MUST NOT be present on `.new`.
* **docs** or **manifest** MUST be present (exactly one of them).
* **ttl** MUST only be present with a *manifest* as individual announced *docs* must always be pinned.

**Processing:**

1. *Publisher precondition (MUST):* Prior to publishing a `.new` with any `cidv1` (inline or via manifest),
the publisher MUST ensure each CID is discoverable via the DHT by calling Provide(CID) and then
FindProviders(CID) and receiving at least one provider peer ID that is not the publisher’s own.
If not satisfied, retry with backoff before publishing.
When a manifest is used, this applies to the manifest CID itself and to every CID listed within the manifest.
2. *Document validation:* Fetch and pin all CIDs from docs or manifest before insertion.
3. *Atomic pinning:* if any CID cannot be fetched and pinned within the pinning retry window,
   release partial pins and defer insertion.
4. *Recompute SMT root:* Upon successful pin of all CIDs, insert each CID into the local SMT; compute local root.
5. *Verify convergence:* If local root ≠ sender root, mark divergence and enter reconciliation backoff (see State Machines),
   unless parity is achieved during backoff via subsequent `.new`/`.dif` reception.

### .syn (topic `<base>.syn`)

**Semantics:** Solicitation for reconciliation; requests a diff from peers.

**Payload-body Keys:**

| Index | Name | Type | Description |
| --- | --- | --- | --- |
| 1 | **root** | root-hash | BLAKE3-256 of the requesters’s SMT root |
| 2 | **count** | uint | sender’s current document count |
| 3 | **to**   | peer-pubkey | suggested target peer to respond |
| 4 | **prefix**  | array of cidv1<br>*OPTIONAL* | Number of prefix document to validate against.<br>**MUST** be a power of two, with a max size of 16,384. |

**Processing:**

* Any peer MAY respond if it believes it can help reconcile;
  responders SHOULD use jitter (see Timers) and suppress if a suitable `.dif` appears.
* Observers MAY use information to converge opportunistically,
  but `.syn` does not carry updates itself.

#### Determining the number of **prefix** entries in the message

If there are ≤ 64 documents in the tree being solicited, then there are no **prefix** entries (the root suffices).
If there are > 64, include a **prefix** array of length 2^D, where D ≥ 1.

Target bucket size is ~64 documents. When total docs N passes 64×2^D, increase depth to D+1 so the new average bucket size becomes ≈ N/2^(D+1) (~half the previous, trending back toward 64 as documents grow). This produces a stable oscillation between ~32 and ~64 per bucket and keeps reconciliation overhead predictable.

**Prefix Entries:**

Prefix entries are the SMT node hashes at depth D across the tree from left to right (hence a power-of-two count). Each increment of D doubles the number of prefix hashes. For message-size constraints, D MUST NOT exceed 14 (max 16384 entries); deeper arrays risk exceeding the 1 MiB limit.

#### CDDL — `.syn` payload-body

```cddl
; Payload body fits within the Common Message Envelope
payload-body = msg-syn

; numeric keys
root = 1
count = 2
to = 3
prefix = 4

msg-syn = {
  root => root-hash,
  count => uint,
  ? to => peer-pubkey,
  ? prefix => prefix-array
}

prefix-array =
    [ 2*2 prefix-hash ]
  / [ 4*4 prefix-hash ]
  / [ 8*8 prefix-hash ]
  / [ 16*16 prefix-hash ]
  / [ 32*32 prefix-hash ]
  / [ 64*64 prefix-hash ]
  / [ 128*128 prefix-hash ]
  / [ 256*256 prefix-hash ]
  / [ 512*512 prefix-hash ]
  / [ 1024*1024 prefix-hash ]
  / [ 2048*2048 prefix-hash ]
  / [ 4096*4096 prefix-hash ]
  / [ 8192*8192 prefix-hash ]
  / [ 16384*16384 prefix-hash ]

; self-contained types
blake3-256 = bytes .size 32 ; BLAKE3-256 output
root-hash = blake3-256      ; Root hash of the Sparse Merkle Tree
prefix-hash = blake3-256    ; Node hash at depth D (prefix bucket) of the SMT
ed25519-pubkey = bytes .size 32
peer-pubkey = ed25519-pubkey
```

Diagnostic example (payload-body decoded):

```cbor
{ 
  1: h'aaaa...aaaa', 
  2: 100, 
  3: h'cafebabe' 
}
```

Diagram — Sync Handshake

```mermaid
sequenceDiagram
  autonumber
  participant R as Requester
  participant D as DHT
  participant S as Responder(s)
  participant PS as PubSub
  R->>PS: .syn [root_R, count_R]
  PS-->>S: .syn
  rect rgb(245,245,245)
    Note over S: Pre-publish check for .dif
    S->>D: Provide(docs/manifest CIDs)
    S->>D: FindProviders(CID)
    D-->>S: provider ≠ S (OK)
  end
  S->>PS: .dif [in_reply_to, docs/manifest, root_S, count_S, ttl?]
  PS-->>R: .dif
  R->>R: fetch+pin, update SMT → parity
```

### .dif (topic `<base>.dif`)

**Semantics:** Reconciliation reply;
carries the same Document Dissemination payload as `.new`, but is sent specifically in response to a `.syn`.

**Payload-body:** Uses the shared Document Dissemination payload (see “Document Dissemination”).

**Topic rules:**

* in_reply_to MUST be present and MUST reference the `.syn` being answered (its envelope seq).
* docs or manifest MUST be present (exactly one of them).
* ttl SHOULD be present when manifest is used; responders MUST keep manifest blocks available for at least ttl seconds.

Prefix-depth selection for bucketization

* To keep diff sizes predictable, responders partition candidate CIDs by the
  first d bits of `BLAKE3-256(CIDv1-bytes)` (prefix buckets).
* Depth d is chosen based on the responder’s document count `N` to target ≤ 64 items per bucket until a maximum depth is reached:
  * Compute `d_req = ceil(log2(max(1, N / 64)))`.
  * Set `d = min(15, max(1, d_req))`.
    * Minimum depth 1 (root plus two buckets).
    * Maximum depth 15 (root plus 32k buckets); beyond 15, buckets may exceed 64 items.
* When a manifest is used, the chosen `d` MUST be recorded in the manifest as `prefix_depth`
  so receivers can reason about bucket sizing and structure.
* Bucketization only affects grouping/chunking for transfer; receivers still fetch/pin CIDs and update their SMT normally.

**Processing:**

Publisher precondition (MUST): Prior to publishing a `.dif` with any `cidv1` (inline or via manifest), the responder MUST ensure each CID is discoverable via the DHT by calling Provide(CID) and then FindProviders(CID) and receiving at least one provider peer ID that is not the responder’s own.
If not satisfied, retry with backoff before publishing.
When a manifest is used, this applies to the manifest CID itself and to every CID listed within the manifest.

* Requesters fetch+pin any CIDs listed inline or in the diff manifest, update SMT, and check parity.
* Observers MAY also use `.dif` to converge faster.

### .prv (topic `<base>.prv`, OPTIONAL)

**Semantics:** Request SMT inclusion proof(s) for a specific CID from one or more peers.

**Payload-body Keys:**

* **root** (1): root32 — requester’s current root
* **count** (2): uint — requester’s current count
* **cid** (3): cid1 — the document CID requested
* **provers** (4) OPTIONAL: array of peer-pubkey — explicit peers asked to respond
* **hpke_pkR** (5): bytes .size 32 — requester’s ephemeral X25519 public key (REQUIRED)
* Precondition (MUST): Before publishing `.prv` that references `cid`, the requester MUST ensure FindProviders(cid) returns at least one provider peer ID that is not the requester’s own; otherwise retry with backoff.
  This avoids soliciting proofs for undiscoverable content.
* Processing:
  * If `provers` is present, only listed peers SHOULD answer; others SHOULD ignore to avoid unnecessary replies.
  * If `provers` is absent, any peer MAY volunteer a proof after responder jitter;
    responders DO NOT suppress based on other `.prf` replies (multiple independent proofs are acceptable).
  * `.prv` carries no updates by itself.

CDDL — `.prv` payload-body

```cddl
; self-contained types
root32 = bytes .size 32
cid1 = bytes
peer-pubkey = bytes .size 32

msg-prv = payload-body
; numeric keys
root = 1
count = 2
cid = 3
provers = 4
hpke_pkR = 5

payload-body = {
  root => root32,
  count => uint,
  cid => cid1,
  ? provers => [* peer-pubkey],
  hpke_pkR => bytes .size 32
}
```

Diagnostic example (payload-body decoded):

```cbor
{ 1: h'dddd...dddd', 2: 200, 3: h'05e8...cid1', 4: [ h'aa11bb22', h'cc33dd44' ], 5: h'5566...' }
```

### .prf (topic `<base>.prf`, OPTIONAL)

**Semantics:** Reply to a `.prv` with an SMT inclusion proof for the requested `cid`.

**Payload-body Keys:**

* **root** (1): root32 — responder’s current root at proof time
* **count** (2): uint — responder’s current count
* **in_reply_to** (3): uuid — UUIDv7 of the `.prv` being answered
* **cid** (4): cid1 — the requested document CID
* **hpke_enc** (5): bytes .size 32 — responder’s HPKE encapsulated ephemeral public key (REQUIRED)
* **ct** (6): bytes — HPKE ciphertext of the proof payload (REQUIRED)
* Processing:
  * Only the requester possessing the matching X25519 private key can decrypt `ct`.
  * After decryption, verify bindings and the SMT proof; see Encrypted Proofs.
  * Non-requesters cannot decrypt and SHOULD ignore the ciphertext.

CDDL — `.prf` payload-body and encrypted plaintext

```cddl
; self-contained types
root32 = bytes .size 32
cid1 = bytes
uuid = #6.37(bytes .size 16)

msg-prf = payload-body
; numeric keys
root = 1
count = 2
in_reply_to = 3
cid = 4
hpke_enc = 5
ct = 6

payload-body = {
  root => root32,
  count => uint,
  in_reply_to => uuid,
  cid => cid1,
  hpke_enc => bytes .size 32,  ; hpke-enc
  ct => bytes                  ; ct
}

; Encrypted plaintext structure inside ct
smt-proof = {
  kp-type => uint,             ; 0 incl, 1 excl
  kp-k => bytes .size 32,
  kp-siblings => [* bytes .size 32],
  ? kp-leaf => bytes .size 32,
  ? kp-depth => uint
}

; smt-proof key constants
kp-type = 1
kp-k = 2
kp-siblings = 3
kp-leaf = 4
kp-depth = 5

prf-plaintext = {
  kt-responder => bytes,  ; responder (peer-pubkey)
  kt-in_reply_to => uuid,
  kt-cid => cid1,
  kt-root => root32,
  kt-count => uint,
  kt-present => bool,
  kt-proof => smt-proof
}

; prf-plaintext key constants
kt-responder = 1
kt-in_reply_to = 2
kt-cid = 3
kt-root = 4
kt-count = 5
kt-present = 6
kt-proof = 7
```

Diagnostic example (payload-body decoded):

```cbor
{ 1: h'dddd...dddd', 2: 201, 3: h'018f0f92c3f8a9b2c7d1112233445570', 4: h'05e8...cid1', 5: h'1122...', 6: h'99aa...' }
```

## Proof Topics Usage Model (Optional)

* Roles:
  * Proven storage peers: nodes that commit to answering proof requests.
  * Non-proven peers: nodes that generally do not need proofs but may occasionally request them.
* Recommended subscription pattern:
  * Proven storage peers SHOULD remain subscribed to `<base>.prv` only.
    Upon receiving a `.prv` they intend to answer, they SHOULD temporarily subscribe to `<base>.prf`,
    apply responder jitter, publish their `.prf`, and promptly unsubscribe.
    They DO NOT suppress due to other `.prf` replies; proofs are tied to the responder’s storage commitment.
  * Non-proven peers SHOULD remain unsubscribed from proof topics under normal operation.
    When a proof is needed:
    1. Subscribe to `<base>.prv` and `<base>.prf`.
    2. Publish `.prv` specifying the `cid` (and optionally specific `provers`) and include `hpke_pkR` (ephemeral X25519 public key).
    3. Wait for `.prf` replies, decrypt, verify, and cache as needed.
    4. Unsubscribe from `<base>.prf` (and `<base>.prv` if no further requests).
* Rationale: This pattern effectively narrows `.prf` delivery to the requester and the responding
  prover(s) currently subscribed, approximating point-to-point behavior over pub/sub and reducing
  background load for nodes that do not need proofs.

## State Machines

* Local peer maintains per-remote-peer sync state for each `<base>`:
  * Stable: local root equals last known root for all known peers.
  * Diverged: a mismatch exists (local root ≠ any seen remote root).
    On entering Diverged, start backoff timer.
  * Reconciling: after backoff expiry, publish `.syn`; await suitable `.dif` and apply.
  * Parity achieved: upon local root matching the responder’s advertised root; return to Stable.

* Transitions may be triggered by `.new` or `.dif` arriving during backoff; if parity is reached, abort solicitation.

Diagram — Peer Sync State Machine (per `<base>`)

```mermaid
stateDiagram-v2
  [*] --> Stable
  Stable --> Diverged: root mismatch observed
  Diverged --> Reconciling: backoff timer expires
  Reconciling --> Stable: parity achieved
  Diverged --> Stable: passive convergence via .new/.dif
  Reconciling --> Diverged: new mismatch observed
```

## Timers and Retries

* Backoff/jitter before sending `.syn`: uniform random in `[Tmin, Tmax]` (implementation-configurable; PoC suggestion: 200–800 ms).

* Responder jitter before publishing `.dif`: uniform random in `[Rmin, Rmax]` (PoC suggestion: 50–250 ms).
  Cancel if an adequate `.dif` appears.
* Diff manifest TTL: responders SHOULD keep diff manifest blocks available for at least `TdiffTTL` seconds (default 3600).
  Include the intended `ttl` in `.dif` when possible.
* Pinning retry window: implementations SHOULD configure a bounded retry window `Wpin`
  (e.g., tens of seconds) during which failed CID fetches from a single `.new` announcement are retried;
  if the window elapses without all CIDs pinned, release partial pins and schedule a later retry per node policy.
* Proof reply jitter: responders to `.prv` SHOULD wait a uniform random delay in `[Rmin, Rmax]`
  (same range as `.dif`) while temporarily subscribed to `<base>.prf`, then publish their `.prf`.

## Transport and Size Limits

* Pub/sub messages SHOULD be ≤ 1 MiB.
  Inline arrays of CIDs in `.new`/`.dif` MUST keep total message size ≤ 1 MiB.

* For larger batches or diffs, use manifests referenced by CID.

* Proof topics: `.prf` replies SHOULD respect the same ≤ 1 MiB bound.
  Large proofs (e.g., very deep sibling arrays) are unlikely due to SMT’s fixed size but MAY
  necessitate splitting across multiple `.prf` messages or providing a manifest CID if ever required.

## SMT (Sparse Merkle Tree)

* Purpose: Order-independent, append-only set with inclusion and non-inclusion proofs.

* Keying (PoC restriction): For each document CID, require CIDv1 with multihash function sha2-256 (32-byte digest).
  Extract the multihash digest and use it directly as the SMT key `k` (no re-hashing).
  Non-conforming CIDs MUST be rejected.
* Depth: 256 levels (one per bit of `k`).
* Hash function: BLAKE3-256 (fixed 32-byte output).
* Domain separation:
  * `LeafHash = BLAKE3-256(0x00 || k || 0x01)` (presence-only set; constant value 0x01).
  * `NodeHash = BLAKE3-256(0x01 || left || right)`.
  * `Empty[d]` precomputed per depth: `Empty[256] = BLAKE3-256(0x02)`, `Empty[d] = NodeHash(Empty[d+1], Empty[d+1])`.
* Root: 32-byte `NodeHash` at depth 0 (BLAKE3-256 output).
* Inclusion proof: path bits from `k` plus sibling hashes per level.
  Exclusion proof: proof of `Empty` at divergence depth or neighbor leaf.

Diagram — Simplified SMT Proof Path

```mermaid
graph TD
  R[Root]
  R --> A
  R --> B
  A --> A0
  A --> A1
  A1 --> L[Leaf(k)]
  classDef sib fill:#FFF3E0,stroke:#FB8C00
  classDef leaf fill:#E8F5E9,stroke:#2E7D32
  S0[Sibling hash]:::sib --- A0
  S1[Sibling hash]:::sib --- A1
  class L leaf
```

## Encrypted Proofs (Mandatory for `.prf`)
<!-- markdownlint-disable MD033 -->
* Algorithms (HPKE per RFC 9180 profile):
  * KEM: DHKEM(X25519, HKDF-SHA256)
  * KDF: HKDF-SHA256
  * AEAD: ChaCha20-Poly1305
* Request flow:
  * Requester generates an ephemeral X25519 key pair and publishes `.prv` including `hpke_pkR` (32-byte public key).
* Response flow:
  * Prover derives an HPKE context using `hpke_pkR`, generates `hpke_enc` (32-byte encapsulated pub),
    and encrypts the proof payload into `ct`.
* Ciphertext AAD binding:
* The AEAD additional authenticated data MUST be the deterministic CBOR encoding of the array<br>
  `[peer-pubkey, seq, ver, in_reply_to, cid, root, count]`, where:<br>
  `peer-pubkey, seq, ver` are the first three fields from the envelope and<br>
  `in_reply_to, cid, root, count` are from the payload-body.
  * This prevents transplanting `ct` under a different envelope or payload context.
* Encrypted plaintext fields (see `prf-plaintext` CDDL):
  * kt-responder (1): peer-pubkey — MUST equal the envelope peer-pubkey
  * kt-in_reply_to (2): uuid — MUST match payload-body in_reply_to
  * kt-cid (3): cid1 — MUST match payload-body cid
  * kt-root (4): root32 — MUST match payload-body root
  * kt-count (5): uint — MUST match payload-body count
  * kt-present (6): bool — whether the CID is included
  * kt-proof (7): smt-proof — inclusion/non-inclusion proof
* Verification (requester):
  1. Decapsulate with X25519 private key to obtain AEAD key/nonce, then decrypt `ct` using AAD as above.
  2. Check that `responder`, `in_reply_to`, `cid`, `root`, and `count` exactly match the outer payload fields.
  3. If `present = true`, verify the SMT inclusion proof against `root`; if `false`, verify the non-inclusion proof.
* Non-requesters cannot decrypt and SHOULD ignore `.prf` ciphertext.
<!-- markdownlint-enable MD033 -->

Diagram — Encrypted Proof Exchange (.prv/.prf)

```mermaid
sequenceDiagram
  autonumber
  participant R as Requester
  participant V as Prover (Responder)
  participant PS as PubSub
  Note over R: generate X25519 ephemeral (hpke_pkR)
  R->>PS: .prv [root_R, count_R, cid, hpke_pkR, provers?]
  PS-->>V: .prv
  Note over V: HPKE seal (hpke_enc, ct = Enc(proof))
  V->>PS: .prf [root_V, count_V, in_reply_to, cid, hpke_enc, ct]
  PS-->>R: .prf
  R->>R: HPKE open(ct) → proof
  R->>R: verify proof vs. root_V; record
```

### SMT Proof Encoding

* Proofs use numeric keys with named constants (see CDDL):
  * kp-type (1): uint — 0 = inclusion, 1 = non-inclusion
* kp-k (2): bytes .size 32 — k = sha2-256 multihash digest from the CIDv1 (32 bytes)
  * kp-siblings (3): array of bytes .size 32 — ordered from leaf upward (LSB-first)
  * kp-leaf (4) OPTIONAL: bytes .size 32 — LeafHash; MAY be omitted
  * kp-depth (5) OPTIONAL: uint — defaults to 256 if omitted

CDDL — SMT proofs

```cbor
smt-proof = {
  kp-type => uint,            ; 0 incl, 1 excl
  kp-k => bytes .size 32,     ; k
  kp-siblings => [* bytes .size 32],
  ? kp-leaf => bytes .size 32,
  ? kp-depth => uint
}

kp-type = 1
kp-k = 2
kp-siblings = 3
kp-leaf = 4
kp-depth = 5
```

* Verification procedure (inclusion):
  1. Compute `k` from the provided CID and compare to `proof.k`.
  2. Compute `LeafHash` using domain separation; iteratively hash with `siblings` per bit of `k` to reconstruct a candidate root.
  3. Accept if candidate root equals the responder’s `root` in the `.prf` payload.
* Verification procedure (non-inclusion):
  * Either demonstrate a path leading to an `Empty` node at divergence depth or provide a neighbor leaf proof
    whose key differs at the first differing bit.

## Diff Reconciliation

* Objective: Provide the requester with a complete list of CIDs it may be missing relative to the responder’s advertised snapshot,
  with minimal interaction.
* Flow:
  1. Requester publishes `.syn` with its current `root` and `count` (optionally targeting a specific peer via `to`).
  2. Responder computes the set of CIDs the requester may be missing relative to its own snapshot
     (the responder’s current `root`/`count`).
     Exact determination may rely on local indexes and heuristics; under honest assumptions,
     including all responder-held CIDs suffices for convergence.
  3. If the list is small (≤ 1 MiB when encoded), responder MAY include it inline in `.dif` as `missing_for_requester`.
  4. Otherwise, responder assembles a diff manifest (see Diff Manifest) encoded with deterministic CBOR,
     stores it in IPFS without pinning, and replies on `.dif` with the manifest CID and an
     intended availability `ttl` (default 3600 seconds).
  5. Requester fetches the manifest, pins and inserts any CIDs it does not already have, and updates its SMT.
    Further `.new` or `.dif` messages will drive it to parity.
* Caching:
  * For a given responder snapshot (`root`,`count`), the manifest CID is stable;
    responders SHOULD cache and reuse it across solicitations.
* Availability:
  * Responders SHOULD keep manifest blocks available for at least `TdiffTTL` seconds (default 3600).
    Implementations MAY choose to pin temporarily or serve blocks opportunistically.
* Rate limiting:
  * Responders SHOULD apply jitter and MAY suppress replies if another adequate
    `.dif` is seen for the same `.syn` to limit redundant manifests.
* Prefix depth:
  * Responders SHOULD select and record prefix_depth per the .dif rules above to keep per-bucket sizes ≲ 64 while depth ≤ 15.

Diagram — Prefix Depth Selection

```mermaid
flowchart LR
  N[N = responder count] --> X{N <= 64?}
  X -- Yes --> D1[d = 1]
  X -- No --> C1[compute d_req = ceil(log2(N/64))]
  C1 --> CL[clamp d = min(15, max(1, d_req))]
  CL --> OUT[d used for bucketization\nrecord in manifest as prefix_depth]
```

## Diff Manifest (IPFS object)

* Use when inline lists would exceed 1 MiB.
* Numeric keys are used for fields (named constants):
  * k-ver (1): ver (uint)
  * k-in_reply_to (2): uuid
  * k-responder (3): peer-pubkey
  * k-root (4): root32
  * k-count (5): uint
  * k-missing_req (6): array of cid1
  * k-missing_resp (7) OPTIONAL: array of cid1
  * k-ttl (8) OPTIONAL: uint — seconds the responder intends to keep manifest blocks available (default 3600)
  * k-sig (9): bstr — signature by responder over the manifest body
  * k-prefix_depth (10) OPTIONAL: uint — prefix bucket depth d used for manifest bucketization
* The `.dif` payload carries the manifest CID.

CDDL — Diff manifest

```cbor
diff-manifest = {
  k-ver => ver,
  k-in_reply_to => uuid,
  k-responder => peer-pubkey,
  k-root => root32,
  k-count => uint,
  k-missing_req => [* cid1],
  ? k-missing_resp => [* cid1],
  ? k-ttl => uint,
  k-sig => bstr,
  ? k-prefix_depth => uint
}

k-ver = 1
k-in_reply_to = 2
k-responder = 3
k-root = 4
k-count = 5
k-missing_req = 6
k-missing_resp = 7
k-ttl = 8
k-sig = 9
k-prefix_depth = 10
```

Diagram — Diff Manifest Schema (fields)

```mermaid
classDiagram
  class diff_manifest {
    ver (1): uint
    in_reply_to (2): uuid
    responder (3): peer-pubkey
    root (4): root32
    count (5): uint
    missing_req (6): [cid1]
    missing_resp (7): [cid1] optional
    ttl (8): uint optional
    sig (9): bstr
    prefix_depth (10): uint optional
  }
```

Diagnostic example (decoded):

```cbor
{
  1: 1,
  2: h'018f0f92c3f8a9b2c7d1112233445567',
  3: h'44556677',
  4: h'cccc...cccc',
  5: 200,
  6: [ h'06f9...cid1', h'07aa...cid1' ],
  8: 3600,
  9: h'...sig...'
}
```

## Error Handling

* Invalid signature or CBOR not using deterministic encoding: drop.

* Oversized message: drop.
* Fetch/pin failure: do not insert into SMT; release partial pins from the announcement;
  retain a pending queue and retry within the configured pinning window/policy.

## Security Considerations (PoC)

* Honest participants assumed; messages are public and unauthenticated beyond per-peer signatures,
  except `.prf` proof content which is encrypted end-to-end using HPKE as specified.

* Implementations SHOULD rate-limit `.syn` and `.dif` per peer and bound pin concurrency to avoid resource exhaustion.

## Privacy Considerations (PoC)

* None; all fields are public.
  Future versions may add encryption/signing via COSE.

## Observability and Metrics

* Track: `.new` seen, pins queued/succeeded/failed, roots observed,
  divergence detected, `.syn` sent, `.dif` received,
  diff manifests served/fetched, bytes fetched.
* If proof topics enabled: `.prv` sent/received, `.prf` verified, proof cache hits.

## Interoperability

* Deterministic CBOR encoding is required for all payloads and manifests.
  CIDs MUST be CIDv1; CID arrays must contain their binary representations.
* PoC CID restriction: Document CIDs MUST be CIDv1 with multihash sha2-256 (32-byte digest).
  Other multihash functions are not accepted in this version.
* SMT hashing MUST be BLAKE3-256 exactly as specified; mixing hash functions will produce incompatible roots and proofs.

## Extensibility

* New optional fields may be added to payload maps.
  Unknown optional fields MUST be ignored.

## Conformance and Test Vectors

* Provide fixtures for `.new`, `.syn`, `.dif`, and a small SMT set (TBD in repository).

## References

* Sparse Merkle Trees: RFC 6962 (conceptual), Cosmos ICS23 (proof encoding inspiration).
* BLAKE3: O'Connor et al., <https://github.com/BLAKE3-team/BLAKE3> (specification and reference implementations).
* Deterministic CBOR: RFC 8949, Sections 4.2.1–4.2.3 (length-first core deterministic encoding).
  Tags MUST appear only when specified by the CDDL; otherwise tags are omitted.
* CID/multihash: Document CIDs MUST be CIDv1 with multihash sha2-256 (32-byte digest) in this PoC.
  See <https://github.com/multiformats/cid> and <https://github.com/multiformats/multihash>.

## Open Questions

* Numeric defaults (`Tmin/Tmax`, size caps) may be tuned through experimentation.

* Potential future direct-stream optimization for large diffs.

## Glossary

* **CID**: IPFS Content Identifier.
* **SMT**: Sparse Merkle Tree (append-only presence set over CIDs).
* **Root**: SMT root hash summarizing the entire set.
* **Manifest**: IPFS object (by CID) describing a batch of CIDs or a diff.
* **UUIDv7**: 128-bit, time-ordered unique identifier used as message/correlation id.
