---
icon: material/puzzle
---

# Runtime Extensions

Hermes exposes host capabilities to WASM modules via Runtime Extensions (HREs). Each extension defines WIT interfaces that modules can import and call. Extensions are typically singletons and receive a context for each call that includes the application name, module ID, event name, execution counter, and a VFS handle.

Key characteristics
- WIT-based API definitions with generated Rust bindings.
- Context propagation per call; extensions can register background tasks on first use.
- Capability scoping and input validation to reduce attack surface.

Notable extensions (hermes/bin/src/runtime_extensions/hermes)
- http_gateway: HTTP server, hostname routing, endpoint subscriptions, request classification, static file serving.
- http_request: Module-side API to send HTTP requests via async gateway machinery.
- kv_store: Simple key/value storage patterns.
- sqlite: Embedded SQLite access via host functions.
- ipfs: Publish/subscribe, DHT, file add/get/pin, and peer eviction using an embedded node.
- cardano: Chain following and eventing helpers for Cardano integration.
- crypto: Key management and cryptographic utilities (e.g., BIP39, BIP32-Ed25519).
- cron: Scheduled events for modules.
- logging: Structured logging APIs from modules.

Context hooks
- Extensions can register once-per-process initialization when a context is first observed (e.g., start the HTTP gateway, set up IPFS streams).

See also
- 05_building_block_view/hermes_engine.md
- 08_concepts/event_model.md
