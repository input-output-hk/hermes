---
    title: 0007 Embedded IPFS/libp2p for Pub/Sub, DHT, and Files
    adr:
        author: Steven Johnson <steven.johnson@iohk.io>
        created: 21-Oct-2025
        status:  accepted
        extends:
            - 0001-arch-std
            - 0002-adr
    tags:
        - arc42
        - ADR
---

## Context

Hermes (and Athena) require decentralized distribution primitives:

* Pub/Sub for event streams (e.g., ballot casting and receipts)
* DHT for lightweight state dissemination
* Content‑addressed files for larger payloads (e.g., proposals)

Alternatives considered

* Custom network over TCP/WebSockets: high maintenance, reinvents peer discovery and transport
* Centralized brokers (NATS/Kafka/MQTT): strong tooling but introduces central dependencies

## Decision

Embed an IPFS/libp2p node in each Hermes process to provide pub/sub, DHT, and file distribution capabilities.

## Consequences

Positive

* Decentralized by design; peer discovery and transport handled by libp2p
* Unified API for messages (pub/sub), small key/values (DHT), and large payloads (files)

Trade‑offs and risks

* Untrusted network; must validate topics/messages and evict misbehaving peers
* Operational footprint per node; bootstrap configuration required

## Implementation

* `hermes/bin/src/ipfs/*` boots an embedded node and exposes host APIs to modules
* Per‑application tracking of subscriptions and pins; topic/content validation hooks

## References

* Concepts: [IPFS](../08_concepts/ipfs.md), [Catalyst MVP](../08_concepts/catalyst_mvp.md)

---
