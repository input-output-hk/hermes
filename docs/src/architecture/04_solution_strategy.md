---
icon: material/strategy
---

# Solution Strategy

<!-- See: https://docs.arc42.org/section-4/ -->

| Goal/Requirement | Solution | Details |
| --- | --- | --- |
| Flexible and modular backend engine to run decentralized applications | Event-driven engine on Wasmtime using the WASM Component Model; all host capabilities exposed via WIT | 05_building_block_view/hermes_engine.md |
| WASM application packaging | HDF5-based application containers with strict directory layout, metadata, and immutability | 08_concepts/hermes_packaging_requirements/overview.md |
| Application integrity and trust | COSE signatures over CBOR payloads; certificate store; API version checks | 08_concepts/hermes_signing_procedure/ |
| HTTP and browser integration | Built-in HTTP gateway serving static assets and routing API requests to modules by endpoint subscriptions and app hostnames | 08_concepts/http_gateway.md |
| P2P distribution and coordination | Embedded IPFS/libp2p node for pub/sub, DHT, and content addressing; signature validation on topics/messages | 08_concepts/ipfs.md |
| Efficient WASM execution | Pre-linked `InstancePre` per module; immutable module state across calls via explicit runtime context | 05_building_block_view/hermes_engine.md |
| Data access and isolation | VFS backed by HDF5 with permissioned directories (`srv`, `usr`, `tmp`, etc.) | 08_concepts/vfs.md |
| Event routing and concurrency | Global MPSC event queue + thread pool with per-target dispatch; explicit app/module targeting | 08_concepts/event_model.md |
| Catalyst MVP flows | Topic schema, receipt model, dependency tracking for event processing | 08_concepts/catalyst_mvp.md |
