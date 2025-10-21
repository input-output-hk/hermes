---
icon: fontawesome/solid/biohazard
---

# Risks and Technical Debts

<!-- See: https://docs.arc42.org/section-11/ -->

## Identified risks

* WASM Component Model maturity
  * Risk: Ecosystem and tooling still evolving; potential breaking changes.
  * Mitigation: Encapsulate bindings generation; track upstream; version APIs and packages.

* Security of embedded capabilities
  * Risk: Host APIs expose powerful operations (HTTP, IPFS, storage).
  * Mitigation: Constrain WIT APIs; validate inputs; enforce VFS permissions; signature verification.

* IPFS/libp2p network behavior
  * Risk: Untrusted network; message floods; partitioning.
  * Mitigation: Message validation and peer eviction; topic scoping; backpressure via queue; deploy multiple nodes.

* Performance regressions
  * Risk: High per-call overhead, contention on queue/locks.
  * Mitigation: Pre-linked instances; per-core worker pool; careful synchronization; benchmark features.

* Package trust and revocation
  * Risk: Compromised keys or certificates.
  * Mitigation: Certificate store management; rotate and revoke; verify `kid` and chains; prefer short-lived keys.

* API evolution and compatibility
  * Risk: Breaking changes across Hermes releases.
  * Mitigation: Explicit API version in package metadata; ADRs for changes; deprecation windows where possible.
