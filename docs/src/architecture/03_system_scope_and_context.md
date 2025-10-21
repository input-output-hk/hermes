---
icon: material/telescope
---

# System Scope and Context

<!-- See: https://docs.arc42.org/section-3/ -->

## Business Context

Hermes provides a decentralized application runtime aimed at reducing reliance on centralized backends.
For Project Catalyst MVP (Athena), Hermes powers a P2P-driven voting backend with:

* Organizer-published event metadata (events, objectives, proposals) via pub/sub and DHT.
* Eligible voter lists and vote power publication.
* Ballot casting over per-voter topics with signed receipts.
* Independent community verification via subscriptions and optional local services.

The front-end (e.g., Catalyst “Voices”) consumes Athena organizer and community data and
interacts with Hermes applications over HTTP.

## Technical Context

Primary external interfaces:

* HTTP Gateway: Serves static assets from packages and routes API requests to WASM modules.
  Hostname `app.domain` selects the application.
* IPFS/LibP2P: Embedded node provides pub/sub, DHT, and file distribution.
  Topic naming and message validation enforce signature/trust rules.
* File and data access: Applications read from VFS (backed by HDF5).
  Select write locations are controlled by permissions (e.g., `tmp`, config).
* Blockchain integration: Runtime extension(s) for Cardano chain following, eventing,
  and associated data pipelines (as needed by modules).

Key internal flows:

* Packaging and validation: HDF5 packages are verified (metadata, structure, signatures) before loading.
* Application boot: Reactor initializes, IPFS node bootstraps, VFS is created/mounted,
  modules are instantiated and initialized via WASM exports.
* Event dispatch: External signals (HTTP, IPFS, chain, cron)
  are translated to Hermes events and dispatched to target applications/modules via the event queue.
