---
icon: material/toy-brick-search
---

# Building Blocks View

<!-- See: https://docs.arc42.org/section-5/ -->

## White box Overall System

The Hermes engine is an event-driven runtime hosting multiple applications (WASM components) and system-wide runtime extensions. Applications are packaged as HDF5 files and verified before load. External inputs (HTTP, IPFS/pubsub, chain followers, cron) are translated into events and dispatched to target modules.

Contained building blocks
- CLI: Packaging, signing, verification, and `run` entrypoint for launching applications.
- Reactor: Global orchestrator that manages applications and dispatches events.
- Event Queue: Singleton queue and worker pool for event routing and execution.
- Application & Module runtime: App container with module registry and VFS; module initialization and event entrypoints.
- Runtime Extensions: Host capabilities (HTTP gateway, http-request, kv-store, sqlite, ipfs, cardano, crypto, cron, logging, etc.).
- VFS/HDF5: Read-mostly virtual filesystem over HDF5 with permissions; mounting of package directories.
- IPFS/libp2p: Embedded node for pub/sub, DHT, and file distribution.
- Packaging & Signing: HDF5-based package builder/validator; COSE signing; certificate management.

Important interfaces
- WIT component interfaces defining host APIs and module exports.
- HTTP gateway routing (`/api`, endpoint subscriptions) and static file serving.
- IPFS pub/sub topic schema and DHT key/value usage.

## Level 2

Selected components

### CLI (bin/src/cli)
- Purpose: `run` Hermes node, build/sign/verify packages, module/app utilities.
- Interfaces: `clap`-based command-line; calls engine services (packaging, reactor, ipfs bootstrap).
- Location: `hermes/bin/src/cli/*`.

### Reactor (bin/src/reactor.rs)
- Purpose: Initialize system, register and manage applications, coordinate shutdown.
- Interfaces: Event queue, application registry, load/unload.

### Event Queue (bin/src/event/queue.rs)
- Purpose: Central dispatch via MPSC channel; filter by target app/module; manage shutdown.
- Interfaces: `send(HermesEvent)`, `shutdown(ExitCode)`, `ExitLock` synchronization.

### Application & Modules (bin/src/app.rs, bin/src/wasm/*)
- Purpose: Own VFS and module instances; initialize and dispatch events to modules.
- Interfaces: WASM component exports (e.g., init, event handlers); runtime context per call.

### Runtime Extensions (bin/src/runtime_extensions/hermes/*)
- Purpose: Provide host APIs for HTTP gateway, http-request, kv-store, sqlite, ipfs, cardano, crypto, cron, logging, etc.
- Interfaces: WIT-defined host traits; context hooks on each call; singleton services as needed.

### VFS/HDF5 (bin/src/vfs/*, bin/src/hdf5/*)
- Purpose: Virtual filesystem with permissioned areas; mounting of package `srv`, `lib`, `usr` trees.
- Interfaces: Read/write APIs; bootstrapping to create HDF5-backed structures.

### IPFS (bin/src/ipfs/*)
- Purpose: Start embedded IPFS node; expose pub/sub, DHT, file add/get/pin.
- Interfaces: Runtime extension host funcs; per-app tracking of subscriptions and pins.

### Packaging & Signing (bin/src/packaging/*)
- Purpose: Build/validate application and module packages; compute hashes; COSE signing and certificate stores.
- Interfaces: CLI subcommands, app builder, author payloads, schema validation.

## Level 3

Examples

### HTTP routing
- `runtime_extensions/hermes/http_gateway`: endpoint subscription loading, hostname parsing, request classification, routing to WASM or static files.

### WASM execution
- `wasm/module.rs`: pre-instantiation, linker setup, runtime context creation per call, init handling via generated exports.

### Event lifecycle
- `event/mod.rs`, `event/queue.rs`, `app.rs`: event construction, queueing, targeted dispatch, per-module execution via thread pool.
