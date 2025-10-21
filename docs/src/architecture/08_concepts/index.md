---
icon: material/thought-bubble
---


# Cross-cutting Concepts

<!-- See: https://docs.arc42.org/section-8/ -->

This section summarizes concepts that cut across many parts of the system:

- Application packaging and signatures: See `hermes_packaging_requirements` and `hermes_signing_procedure`.
- Runtime extensions and WIT interfaces: Host capabilities and how modules call into the engine.
- HTTP gateway: Routing model, endpoint subscriptions, and static asset serving.
- Event model and concurrency: Event queue, targeted dispatch, and dependency tracking.
- Virtual filesystem (VFS): HDF5-backed structure and permission model.
- IPFS/libp2p: Topic schema, DHT, message validation.
- Catalyst MVP: Pub/sub topics, receipts, and trust model applied to Hermes.

See also:
- 05_building_block_view/hermes_engine.md for top-level components.
- 04_solution_strategy.md for rationale and trade-offs.
