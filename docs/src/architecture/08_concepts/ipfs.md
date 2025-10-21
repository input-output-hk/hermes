---
icon: material/transit-connection
---

# IPFS / libp2p

Hermes embeds an IPFS/libp2p node to provide pub/sub messaging, DHT key/value storage, and file distribution.

Capabilities
- Pub/Sub: Subscribe to topics and receive messages; publish messages on topics.
- DHT: Put/get key-value entries for lightweight coordination.
- Files: Add/get/pin content-addressed files; track pins per application.

Topic and message validation (MVP intent)
- Topics follow a structured scheme (e.g., `hash("<app>/<pubkey>/...")`) so validators can verify that messages are signed by the corresponding key.
- Nodes should drop invalid or unauthorized messages; repeated offenders can be evicted.

Engine integration
- IPFS node is bootstrapped at engine start and uses a local data directory under `~/.hermes/ipfs`.
- Host APIs expose publish/subscribe, DHT operations, and file operations to WASM modules.
- Per-application state tracks subscriptions, handles, and pins.

References
- `hermes/bin/src/ipfs/*`
- HTTP-related host APIs: `runtime_extensions/hermes/http_request`
