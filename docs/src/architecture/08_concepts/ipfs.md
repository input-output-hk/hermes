---
icon: material/transit-connection
---

# IPFS / libp2p

Hermes embeds an IPFS/libp2p node to provide pub/sub messaging, DHT key/value storage, and file distribution.

Capabilities

* Pub/Sub: Subscribe to topics and receive messages; publish messages on topics.
* DHT: Put/get key-value entries for lightweight coordination.
* Files: Add/get/pin content-addressed files; track pins per application.

Topic and message validation

* Current validation is minimal (non-empty payload checks).
* Topic-based signature validation is planned but not implemented yet.

Engine integration

* IPFS node is bootstrapped at engine start and uses a local data directory under `~/.hermes/ipfs`.
* Host APIs expose publish/subscribe, DHT operations, and file operations to WASM modules.
* Per-application state tracks subscriptions, handles, and pins.
* Pub/sub requires connecting to other Hermes nodes (custom bootstrap peers) to form a mesh.

Configuration

* `IPFS_BOOTSTRAP_PEERS` (comma-separated multiaddrs) sets custom bootstrap peers.
* `IPFS_LISTEN_PORT` controls the TCP listen port (default: 4001).
* `IPFS_ANNOUNCE_ADDRESS` sets the multiaddr advertised to peers.
* `IPFS_RETRY_INTERVAL_SECS` and `IPFS_MAX_RETRIES` control bootstrap retry behavior.

References

* `hermes/bin/src/ipfs/*`
* HTTP-related host APIs: `runtime_extensions/hermes/http_request`
