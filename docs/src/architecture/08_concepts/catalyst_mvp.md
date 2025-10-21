---
icon: material/vote
---

# Athena (Catalyst MVP) on Hermes

This document captures Athena — the first backend voting application for Project Catalyst —
and its MVP design atop Hermes using IPFS/libp2p and WASM modules.

Objectives

* Replace centralized, heavy backends with decentralized, event-driven components.
* Use pub/sub and DHT to distribute event metadata, voter lists, ballots, and receipts.
* Provide transparent, verifiable receipt issuance for cast ballots.

Topic schema (Athena examples)

* `catalyst/<org_pubkey>/events`: List of past/present/upcoming events.
* `catalyst/<org_pubkey>/event/<ev_id>`: Details of a specific event.
* `catalyst/<org_pubkey>/event/<ev_id>/objectives`: Objectives for an event.
* `catalyst/<org_pubkey>/event/<ev_id>/<obj_id>/proposals`: Proposals for an objective.
* `catalyst/<org_pubkey>/event/<ev_id>/registrations`: Registered voters and vote power.
* `catalyst/<vote_pubkey>/<org_pubkey>/cast/<ev_id>/<obj_id>`: Ballot cast topic (per-voter namespace).
* `catalyst/<org_pubkey>/<vote_pubkey>/receipt/<ev_id>/<obj_id>`: Athena-issued receipt for a recorded ballot.

Trust and validation

* Topics and messages are signed; receivers validate signatures against the topic namespace and known keys.
* Nodes drop invalid messages and can evict misbehaving peers.
* Independent validation (e.g., local vote power calculation) is possible by running local services.

Dependency tracking

* Certain event sources (e.g., a block then its transactions) require parent-before-child processing.
  The MVP leverages per-source queues and interlocks in the event pipeline to avoid blocking unrelated sources
  while preserving necessary order.

Implementation notes

* Hermes modules implement the business logic; runtime extensions provide IPFS, HTTP, crypto, and storage primitives.
* Static assets for frontends can be served from the app package; frontends (e.g., Voices) interact with Hermes over HTTP.

Sources

* Based on “Catalyst - Hermes Core Design” deck (Oct 2025) and Hermes engine source code.
