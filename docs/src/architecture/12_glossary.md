---
icon: material/format-list-group-plus
---

# Glossary

<!-- See: https://docs.arc42.org/section-12/ -->

| Term | Definition |
|------|------------|
| Hermes | Event-driven runtime that executes WASM components packaged as applications. |
| WASM Component Model | Standard for composable WASM components with WIT-defined interfaces. |
| WIT | WebAssembly Interface Types language used to describe host APIs and component interfaces. |
| HDF5 | Hierarchical Data Format v5, used as the application/package container and VFS backing store. |
| VFS | Virtual filesystem presented to applications, backed by HDF5 with permissioned directories. |
| Runtime Extension (HRE) | Engine-provided host capability accessible to modules (HTTP, IPFS, sqlite, crypto, etc.). |
| Reactor | Global orchestrator coordinating applications and event dispatch. |
| Event Queue | Singleton MPSC queue for routing events to target applications/modules. |
| IPFS/libp2p | Peer-to-peer system providing pub/sub, DHT, and content-addressed storage. |
| COSE/CBOR | Standards used for signing and encoding application metadata and author payloads. |
| Catalyst (Athena) | Project Catalyst voting platform leveraging Hermes for decentralized backend. |
