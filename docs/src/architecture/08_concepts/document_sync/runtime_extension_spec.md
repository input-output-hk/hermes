---
icon: material/puzzle
---

# Hermes Runtime Extension: Document Sync — Specification (Draft)

This document specifies the Hermes runtime extension that exposes the Document Sync capability to WASM modules. The extension is intentionally thin: it wraps a reusable, general‑purpose Document Sync core module and adds a stable WIT API, runtime resource management, and host integration specific to Hermes.

The wire protocol, reconciliation algorithm, and message formats are defined in the companion IPFS Document Sync Protocol spec. The runtime extension maps API calls and events to those protocol behaviors and enforces operational policies (pinning, DHT provider checks, quotas, and lifecycle) on behalf of guest modules.

For the protocol details, see: docs/src/architecture/08_concepts/document_sync/protocol_spec.md

## Status

- Stage: PoC draft
- Version: 0.1.0 (aligned with protocol v0.1.0; WIT may evolve)

## Architecture

The runtime extension is split into two components:

1) Core Protocol Module (external, reusable)
- Location: https://github.com//input-output-hk/catalyst-libs/rust/hermes-ipfs
- Responsibilities:
  - Implements the IPFS Document Sync protocol: topic management, message encode/decode, set reconciliation using SMT, diff manifests, and optional proof flows (.prv/.prf).
  - Maintains per‑peer/per‑channel state machines (Stable, Diverged, Reconciling) and timers (backoff, jitter) as defined in the protocol spec.
  - Performs deterministic CBOR framing, signing/verification, message de‑duplication, and protocol version checks.
  - Provides a general API over IPFS/libp2p that can be reused outside Hermes.

2) Hermes Runtime Extension (this crate)
- Responsibilities:
  - Exposes a stable WIT interface to guest WASM modules.
  - Owns resource management: channel handles, reference counting, pin lifetimes, manifest TTL enforcement, backpressure/quotas, and isolation between modules.
  - Adapts host configuration and capabilities (IPFS node, DHT, storage) to the Core module.
  - Forwards API calls and events between guests and the Core with minimal transformation.
  - Provides observability and policy enforcement (limits, validation, permission checks).

Design principle: keep the extension a thin wrapper. All protocol logic belongs in the Core module; Hermes‑specific concerns stay in the wrapper.

## WIT Interfaces

The WIT surface for the extension lives under wasm/wasi/wit/deps/hermes-doc-sync and is rendered to markdown for reference. Linkable references will be added when generated; placeholders are used here:

- API: [Hermes Doc Sync WIT — API]([WIT-API-MD-PLACEHOLDER])
- Events: [Hermes Doc Sync WIT — Events]([WIT-EVENT-MD-PLACEHOLDER])

Source WIT files (for orientation only):
- wasm/wasi/wit/deps/hermes-doc-sync/api.wit
- wasm/wasi/wit/deps/hermes-doc-sync/event.wit
- wasm/wasi/wit/deps/hermes-doc-sync/world.wit

## Scope

In‑scope (runtime wrapper):
- Present a per‑channel API to post/fetch documents and request storage proofs.
- Deliver new‑document events to guests.
- Manage IPFS resources on behalf of guests: DHT provider checks before publish, pin/fetch, manifest TTL, and topic subscriptions per protocol.
- Enforce quotas and lifecycle; isolate channels across modules.

Out‑of‑scope (delegated to Core):
- Protocol semantics: message formats, reconciliation algorithm, proof encryption (HPKE), and state machines.
- Low‑level IPFS/libp2p operations and wire behavior.

## API Surface (semantics)

The API is defined in WIT and exported to guests. The following summarizes semantics and how calls map to protocol behavior. Refer to the generated WIT markdown for exact signatures.

- id-for(doc: doc-data) -> doc-loc
  - Computes the content identifier for the binary document as a CIDv1 (opaque bstr).
  - Host policy: reject non‑conforming CIDs if the configured scheme requires sha2‑256 digest length.

- resource sync-channel
  - constructor(name: channel-name)
    - Opens or joins a document sync channel named `name`.
    - Effect: ensures subscriptions to `<base>.new` and `<base>.syn` for that channel per protocol; `<base>.dif` is subscribed only during reconciliation.
    - Returns a resource handle; multiple modules may hold handles to the same name (sharing the underlying Core instance).

  - close(name: channel-name) -> result<bool, errno>
    - Requests shutdown of the channel `name`. The runtime defers actual teardown until the last handle to that channel is dropped (reference‑counted) to avoid breaking other guests.
    - On final close: unsubscribes from `<base>.dif`, `<base>.syn`, `<base>.new` and releases pins created solely for this channel, subject to retention policy.

  - post(doc: doc-data) -> result<doc-loc, errno>
    - Publishes `doc` into the channel’s set. Behavior maps to `.new` in the protocol:
      1) Compute `doc` CID; 2) Provide(CID) and FindProviders(CID) != self; 3) Pin; 4) Insert into SMT; 5) Broadcast `.new` with inline docs or manifest depending on size.
    - Returns the `doc-loc` (CID) on success.

  - get(loc: doc-loc) -> result<doc-data, errno>
    - Retrieves the document by CID from local storage or via IPFS fetch+pin if not present.
    - Validates payload length against quota before materializing in guest memory.

  - prove-includes(loc: doc-loc, provers: list<prover-id>) -> result<list<doc-proof>, errno>
    - Initiates a proof request flow for inclusion per protocol (`.prv`/`.prf`). If `provers` is empty, requests from all known provers.
    - The Core performs HPKE encryption negotiation and proof verification; the runtime returns a list of validated proofs (opaque bstr) tagged by the embedded prover identity.

  - prove-excludes(loc: doc-loc, provers: list<prover-id>) -> result<list<doc-proof>, errno>
    - As above, but for non‑inclusion/exclusion proofs where supported by the Core.

Notes on WIT shape: some operations are declared `static` in the current WIT. Semantically they operate on the channel instance created by `constructor(name)`. The final WIT may evolve to instance methods; the semantics above are authoritative.

## Events

The extension exports an event interface for new document arrivals:

- on-new-doc(channel: channel-name, doc: doc-data)
  - Delivered when a peer’s `.new` or `.dif` results in a newly pinned document for `channel`.
  - Delivery is at‑least‑once. Guests SHOULD de‑duplicate by `id-for(doc)`.
  - Ordering is best‑effort; multiple documents from the same batch may deliver in any order.
  - The runtime MAY coalesce or rate‑limit events under load.

See: [Hermes Doc Sync WIT — Events]([WIT-EVENT-MD-PLACEHOLDER])

## Lifecycle

- Channel open: first `sync-channel` handle for a `name` creates or attaches to a Core instance. Subscribes to `.new`/`.syn`; `.dif` is subscribed during reconciliation windows only.
- Handle sharing: all handles with the same `name` share the underlying channel state.
- Channel close: `close(name)` marks the channel for shutdown; actual teardown occurs when the last handle is dropped. Retention policy may keep pins shared with other channels or system caches.
- Process shutdown: all channels are closed; the Core flushes pending `.dif` manifests it is obligated to serve until TTL expiry where feasible.

## Resource Management

- Pinning and retention:
  - The runtime pins documents published or fetched as part of reconciliation. If a diff manifest is used, its CID and all listed CIDs are made available for at least `ttl` seconds as announced.
  - Partial pins on a batch failure are released atomically after a bounded retry window.

- DHT availability precheck (mandatory):
  - Before publishing any announcement that references a CID (inline or in a manifest), the runtime provides the CID to the DHT and verifies FindProviders(CID) returns at least one peer other than self.

- Topic subscriptions:
  - Always subscribe to `<base>.new`/`<base>.syn`.
  - Subscribe to `<base>.dif` only during reconciliation; unsubscribe on parity.
  - Proof topics are opt‑in and ephemeral per request.

- Quotas and limits (configurable defaults):
  - Max message size ≤ 1 MiB; larger sets use manifests.
  - Max concurrent posts per channel; max in‑flight fetches/pins and proofs.
  - Bounded memory for event delivery; backpressure by dropping oldest or applying rate limits with counters exposed via metrics.

## Concurrency and Ordering

- Per‑channel serialization:
  - `post` operations are serialized per channel to avoid SMT races; fetches/pins within a batch may be parallel within configured concurrency.

- Cross‑channel concurrency:
  - Independent channels operate concurrently.

- Events:
  - Delivered at‑least‑once; no strict ordering guarantees across different sources. Guests should be idempotent (prefer `id-for` to deduplicate).

## Error Model

The WIT surface has a placeholder errno. The runtime will map underlying errors into a stable taxonomy suitable for guests. Tentative categories:

- invalid-argument: malformed `doc-data` or `doc-loc`.
- channel-not-found / already-closed: misuse of lifecycle.
- quota-exceeded: memory, bandwidth, or concurrency limits exceeded.
- network-timeout / dht-unavailable / pubsub-failed.
- pin-failed / content-not-found.
- proof-verification-failed: `.prf` invalid or cannot decrypt/verify.
- internal-error: unexpected runtime failure.

Exact enumerants will be finalized with the WIT reference ([WIT-API-MD-PLACEHOLDER]).

## Configuration

Configurable parameters applied by the runtime to the Core (defaults match the protocol spec recommendations):

- Backoff/jitter for `.syn` and `.dif` responders (ranges in ms).
- Diff manifest TTL minimums and honoring announced `ttl`.
- Pinning retry window and concurrency limits.
- DHT provider precheck enabled (mandatory) and retry policy.
- Target bucket size for `.syn` prefix selection (≈64 docs) and max depth.
- Topic QoS priorities and per‑topic rate limits (favor `.new`/`.syn`).
- Max message size (1 MiB) and manifest threshold.
- Storage paths/quotas for pins and manifests; retention policy on close.

## Security Considerations

- Message envelope verification and version checks are performed in the Core.
- HPKE is mandatory for proofs; the runtime/Core manage ephemeral keys for `.prv`/`.prf` and verify before returning proofs to guests.
- Input validation: size limits on `doc-data`, `doc-proof`, and list lengths; reject oversized or malformed payloads before allocation.
- Isolation: per‑channel states are isolated; events are scoped by channel name; guests cannot read/write other channels unless explicitly opened.

## Privacy Considerations

- Document announcements are broadcast on pub/sub; no confidentiality is provided by the protocol (except proof ciphertexts). Guests should not place sensitive data in documents unless separately encrypted at the application layer.
- Proofs are encrypted end‑to‑end per protocol; the runtime does not leak plaintext proofs across channels.

## Observability

- Metrics: per‑channel counters for `.new`/`.syn`/`.dif` seen/sent, pin success/failure, DHT prechecks, proof requests/replies, event deliveries, and drops due to backpressure.
- Logs: lifecycle operations, errors with stable codes, and reconciliation state transitions (Stable/Diverged/Reconciling).
- Tracing: spans around `post`, fetch/pin, reconciliation rounds, and proof flows.

## Examples (informative)

Pseudo‑flow for a guest module:

1) Open a channel
   - `let ch = sync-channel::constructor("my-docs")`
2) Post a document
   - `let id = sync-channel::post(doc_bytes)?`
3) React to new documents
   - `on-new-doc("my-docs", doc_bytes)` → `let id = id-for(doc_bytes)`
4) Request an inclusion proof
   - `let proofs = sync-channel::prove-includes(id, [])?`
5) Fetch a document later by id
   - `let doc = sync-channel::get(id)?`
6) Close the channel when finished
   - `sync-channel::close("my-docs")`

See the generated WIT reference for exact signatures: [WIT-API-MD-PLACEHOLDER]

## Relationship to Protocol Spec

This extension implements the runtime surface of the IPFS Document Sync Protocol:

- post → `.new` (and `.dif` indirectly via other peers)
- automatic reconciliation → `.syn`/`.dif` per state machine
- proofs → `.prv`/`.prf` (encrypted, verified)
- event delivery → new local pins resulting from `.new`/`.dif`

The Core module encapsulates these mechanics; the runtime configures and hosts it, enforcing limits and lifecycle.

## Test Plan (high‑level)

- Unit tests: CID computation, validation, quota enforcement, and error mapping.
- Integration tests (with IPFS): posting, reconciliation to parity under churn, manifest TTL honoring, and proof round‑trips with HPKE.
- Conformance: fixtures mirroring the protocol spec examples and CDDL where applicable.

## References

- Protocol spec: docs/src/architecture/08_concepts/document_sync/protocol_spec.md
- WIT sources: wasm/wasi/wit/deps/hermes-doc-sync
- CIDv1, multicodec, multihash (see protocol spec references)

## Open Questions

- WIT shape: should `post`, `get`, and proof functions be instance methods on `sync-channel`? Current WIT marks them `static` yet they semantically act on a channel.
- Error enumeration: finalize stable errno values and mapping from Core/host errors.
- Event delivery contracts: optional batching/coalescing vs. per‑doc events; backpressure behavior exposed to guests.
