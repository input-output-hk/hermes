---
icon: octicons/goal-24
---

# Introduction and Goals

<!-- See: https://docs.arc42.org/section-1/ -->

## Requirements Overview

Hermes is a modular, event-driven execution engine for decentralized applications built on
WebAssembly (WASM) using the WASM Component Model.
Applications are packaged as signed, immutable bundles and executed via a pluggable set of
runtime extensions that provide capabilities such as HTTP,
storage, cryptography, IPFS/libp2p, and blockchain integration.

MVP goals (Project Catalyst):

* Provide a performant, decentralized backend for Catalyst voting built on Hermes.
* Support P2P pub/sub channels and DHT for distributing event metadata, voter lists, ballots, and receipts.
* Enable easy app development using standard Web tooling (HTML/CSS/JS) and/or Flutter, with business logic in WASM components.
* Keep complex concerns (crypto, distributed networking, data persistence)
  in the engine/runtime extensions so application authors focus on UX and core logic.

Functional scope:

* Execute WASM components, dispatch events, and mediate host calls defined via WIT interfaces.
* Package/validate/run applications from HDF5-based bundles with COSE signatures and certificate-based trust.
* Serve static assets and route HTTP requests to WASM module endpoints.
* Integrate IPFS/libp2p for pub/sub, DHT, and file distribution.
* Provide data access via a virtual filesystem backed by HDF5 and policy-controlled permissions.

Out of scope (MVP):

* Fully generalized API coverage for all extensions (only sufficient for MVP needs).
* Multi-tenant resource isolation beyond WASM sandbox and permissioned VFS.
* Full-blown on-chain settlement or rewards distribution flows.

## Quality Goals

* Modularity and extensibility: Clear separation between engine, runtime extensions, and applications;
  WASM component-driven interfaces.
* Security and integrity: Signed, immutable application packages; constrained host APIs; certificate-based trust;
  topic-level signature validation strategy for P2P.
* Performance and scalability: Pre-linked WASM instances, event queue + thread pool, per-core parallelism,
  minimal data copies.
* Reliability and robustness: Explicit init/fini lifecycle, package validation,
  deterministic event dispatch ordering per source, and graceful shutdown.
* Developer ergonomics: WIT-defined interfaces, standard Web tooling for frontends,
  clean CLI for packaging/running/signing.
* Observability: Structured logging/tracing, explicit build info, controlled log levels.

## Stakeholders

| Role | Contact | Expectations |
|------|---------|--------------|
| Platform maintainers (Hermes Core) | IOG Engineering | Maintainable, testable engine with clear extension points and docs |
| Application developers | Community + IOG teams | Stable WIT APIs, easy packaging, predictable runtime and HTTP/IPFS integration |
| Catalyst organizers (Athena) | Catalyst Ops | Reliable P2P distribution, verifiable data, performance at voting scale |
| Catalyst community (voters) | Public | Transparent, receipt-based proof-of-recording, privacy, availability |
| Security reviewers | Internal/External | Clear trust model, signatures, constrained capabilities, auditable flows |
| DevOps/SRE | Internal/Community | Observable runtime, deployable as a service, simple configuration |
