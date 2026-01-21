---
icon: material/server-network
---

# Deployment View

<!-- See: https://docs.arc42.org/section-7/ -->

## Infrastructure Level 1

Hermes runs as a single-node service hosting one or more applications concurrently.
Each node embeds an IPFS/libp2p instance and an HTTP gateway.

Motivation

* Minimize operational dependencies by embedding the IPFS node and HTTP gateway.
* Scale horizontally by running multiple nodes, each hosting apps and participating in pub/sub/DHT.

Quality/Performance Features

* Per-core execution via worker pool; pre-linked WASM modules for low-latency calls.
* Static asset serving from VFS; backpressure via event queue.

Mapping of Building Blocks to Infrastructure

* `HTTP Gateway` → exposed on a configurable port; reverse proxy can front it if required.
* `IPFS/libp2p` → listens on local interfaces; participates in DHT and pub/sub.
* `Reactor/Event Queue` → internal process-only.
* `VFS/HDF5` → local filesystem for package state and app storage; per-app `.hfs` file in Hermes home.

## Infrastructure Level 2

### Single-node deployment

* Run `hermes run athena.happ [--cert <cert.pem> ...]` (Athena) or any `<app>.happ`.

* Suitable for development and small-scale demos.

### Multi-node deployment

* Run multiple Hermes nodes with the same application package (e.g., Athena) to participate in the same IPFS/pubsub topics.

* Front HTTP gateway with a standard reverse proxy (e.g., Nginx) if TLS termination or multi-domain routing is needed.

### Persistence considerations

* Application packages are immutable; only designated VFS areas are writable.
* VFS state persists across runs in `~/.hermes/<app>.hfs` unless cleaned.

* Embedded IPFS uses a local data directory under the Hermes home (`~/.hermes/ipfs`).
