---
icon: material/handcuffs
---

# Architecture Constraints

<!-- See: https://docs.arc42.org/section-2/ -->

## Technical constraints

- Runtime: Rust-based engine using Wasmtime for the WASM Component Model.
- Interface definition: WIT for host capability interfaces and events; bindings generated at build time.
- Packaging: HDF5 (Hierarchical Data Format v5) as the on-disk container for applications and modules.
- Signing and verification: COSE with EdDSA (Ed25519) over CBOR payloads; certificate and key material managed via X.509-compatible formats.
- Networking: IPFS/libp2p for DHT, pub/sub, file distribution; embedded IPFS node spun up by the engine.
- HTTP: Built-in gateway for static content and API routing to WASM modules; hostnames map to application names.
- Storage: Virtual filesystem (VFS) backed by HDF5 with permission levels (read-only vs read-write areas, e.g., `srv`, `usr`, `tmp`).
- Concurrency model: MPSC event queue and a thread pool; WASM components pre-linked into `InstancePre` to reduce per-invocation overhead.
- Languages: Engine and extensions in Rust; WASM components in any language producing WASM components.

## Organizational/process constraints

- MVP prioritization: Only implement the subset of APIs needed for Catalyst voting MVP; keep extension APIs evolvable.
- Backward compatibility: Early-stage APIs may change; application packages include API version metadata for validation.
- Security posture: Packages are immutable; runtime writes limited to designated areas; host APIs intentionally constrained.

## Conventions and standards

- arc42 for architecture documentation; ADRs for key decisions.
- Use WIT for interface contracts and Wasmtime component model for execution.
- Use structured logging with configurable verbosity; avoid panics in runtime code paths.
