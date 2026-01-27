# Hermes + Athena Roadmap — 10‑minute Walkthrough

Speaker: Head of Group
Audience: CEO and executive stakeholders

Opening (30 seconds)

* Our mission this cycle is simple: replace today’s federated side‑chain infrastructure with a
  fully distributed, peer‑to‑peer system that’s auditable, scalable, and resilient.
  That’s what Hermes (our engine) and Athena (the voting experience on top) deliver.
* This aligns with our contract objective and the community proposal:
  decentralize Catalyst infrastructure, support parallel voting events, enable secure Cardano‑based
  vote casting, and make all historic voting data publicly auditable without relying on Web2 services.

What We’re Delivering (60 seconds)

* Hermes Engine: an event‑driven runtime that runs application logic in WebAssembly components and
  provides safe building blocks (HTTP, IPFS/libp2p, SQLite, Cardano, cryptography).
* Athena Voting Interface: the voter experience on top of Hermes, with native modules for identity/registration and Cardano data,
  and a web UI served through the gateway.
* Decentralised by design: content moves over peer‑to‑peer networks instead of relying on central web servers.
* Parallel rounds: multiple funding rounds can run concurrently or overlap without bottlenecks.
* Secure and auditable: votes and history can be verified on‑chain and via immutable, shared data.
* Simple operations: packages are signed, can be fetched from IPFS, and run the same everywhere.
* Product velocity: modular apps let teams ship and evolve features independently.

Roadmap Overview and Dates (30 seconds)

* The work is sequenced into focused milestones, January through May 2026.
  Durations are approximate, and we’ll overlap where practical to accelerate delivery while maintaining quality.
    * Start Jan 2026 (4 wks): WASM Engine Component Model Upgrade
    * Mid Jan 2026 (6 wks): WASM Module Linker Upgrade
    * Start Feb 2026 (8 wks): Event‑Driven Data Validation on IPFS
    * Start March 2026 (4 wks): Parallel WASM Module Execution Framework
    * Mid March 2026 (6 wks): Uniform Resource Management
    * Start April 2026 (6 wks): IPFS Direct Data Readability
    * Start May 2026 (4 wks): Execution From IPFS Link
    * In parallel: Cryptography foundations for Quadratic Voting and Time‑Weighted Stake

Milestone 1 — WASM Engine Component Model Upgrade (Jan, 4 weeks) (75 seconds)

* Plainly: we’re upgrading the “language” our apps speak with the engine so it’s more reliable, safer, and faster to build with.
* We move fully onto the latest WASM Component Model and WIT type definitions.
  That gives us strong, typed boundaries between Hermes and modules, fewer ABI headaches, and better multi‑language support.
* Impact: faster module development, clearer contracts, and consistent behavior across all runtime extensions.
* Demo/readiness: compile example modules with the upgraded bindings; run end‑to‑end through Hermes.

Milestone 2 — WASM Module Linker Upgrade (Mid‑Jan, 6 weeks) (60 seconds)

* Plainly: link only what a module uses.
  Modules shouldn’t need to import every capability or export every event.
* We enhance the linker to support partial linking and graceful traps for unused imports/exports.
  This reduces overhead and makes small, focused modules easy to ship.
* Impact: smaller footprints, faster cold start, simpler modules.
* Demo/readiness: run mixed apps where some modules implement only HTTP, some only IPFS; verify
  unused imports don’t break instantiation.

Milestone 3 — Event‑Driven Data Validation on IPFS (Feb, 8 weeks) (90 seconds)

* Plainly: peers announce and reconcile documents over IPFS; our engine validates those messages before modules act on them.
* We implement the Document Sync flow: structured pub/sub topics for new data, sync, and diffs; per‑channel state;
  and validation so bad messages get dropped.
* Impact: data becomes verifiable, reproducible, and auditable across nodes without central servers.
* Demo/readiness: two Hermes nodes exchange documents; one publishes `.new`, the other reconciles via `.syn/.dif`;
  modules receive on‑new‑document events.

Milestone 4 — Parallel WASM Module Execution Framework (March, 4 weeks) (60 seconds)

* Plainly: keep all CPU cores busy while preserving order where it matters.
* We use a worker pool to run unrelated module calls in parallel, but preserve per‑source ordering
  so dependent streams (like chain blocks) stay correct.
* Impact: higher throughput and responsiveness under load.
* Demo/readiness: stress test mixed HTTP/API traffic and IPFS events; observe parallel execution with ordered per‑source processing.

Milestone 5 — Uniform Resource Management (Mid‑March, 6 weeks) (60 seconds)

* Plainly: a consistent way for modules to open/close handles (like network channels, DB cursors) so we don’t leak resources.
* We standardize resource lifecycle for WIT resources across extensions, with per‑app isolation and reference counting.
* Impact: fewer edge‑case bugs, predictable cleanup, and simpler extension code.
* Demo/readiness: open/close resources from multiple modules in one app; verify isolation and cleanup at app shutdown.

Milestone 6 — IPFS Direct Data Readability (April, 6 weeks) (75 seconds)

* Plainly: read directly from the network when we can, instead of copying everything locally first.
* Hermes apps and packages will be able to consume content from IPFS paths directly, with smart pinning and caching where needed.
  Static web assets can be served this way too.
* Impact: less duplication, faster distribution, and simpler deployments.
* Demo/readiness: serve Athena static assets and fetch shared data via IPFS links; verify pinning and quotas.

Milestone 7 — Execution From IPFS Link (May, 4 weeks) (75 seconds)

* Plainly: start an app from an IPFS CID without pre‑installing it locally.
* We’ll extend the CLI so `hermes run <ipfs path>` fetches, validates signatures, pins as needed,
  and launches the app with the usual safety checks.
* Impact: one‑click, decentralized app distribution; simpler operator workflows.
* Demo/readiness: run Athena from an IPFS link; verify signatures and reproducibility end‑to‑end.

Cross‑Cutting — Cryptography for QV and Time‑Weighted Stake (threaded across Q2) (60 seconds)

* Plainly: we prepare the primitives to support quadratic voting and time‑weighted stake models.
* We extend our cryptography runtime and data paths so votes can be weighted and verified transparently,
  with the right guardrails for privacy and audit.
* Impact: richer governance mechanisms on a decentralized stack.

Athena: What Ships with the Voting Interface (90 seconds)

* We’re delivering the voting experience on top of Hermes.
  The HTTP gateway serves the web UI and routes API calls to native WASM modules.
    * Temporary bridge: today’s `http-proxy` module forwards a few endpoints to external services while we build out native modules;
      it is explicitly marked for deprecation and will be removed once native modules are ready.
    * Native modules we’re shipping:
        * RBAC Registration: look up registration status and related keys for Catalyst IDs and stake addresses.
        * Cardano Indexers: track staked ADA and related chain events via the Cardano runtime extension.
    * The UI is built in Flutter Web and can be served via Hermes’ static file support.

Team and Resourcing (45 seconds)

* With 4 senior Rust engineers and 1 mid‑level:
    * Three seniors focus on Hermes core (WASM upgrades, linker, parallel execution, resource management).
    * One senior leads IPFS + Document Sync and the HTTP gateway hardening.
    * The mid‑level focuses on Athena modules and integration, pairing with seniors for reviews.
* This split lets us overlap milestones while keeping ownership clear.

Quality, Risks, and How We De‑risk (60 seconds)

* Quality: typed interfaces, package signatures, static file serving, and pre‑linked WASM reduce runtime surprises.
  We benchmark and stress test concurrency early.
* Key risks and mitigations:
    * WASM component model/tooling evolution → isolate bindings, version APIs, track upstream (ADR in place).
    * IPFS network behavior → strict message validation, topic scoping, and peer eviction; backpressure via event queue.
    * Performance regressions → per‑core worker pool, pre‑linking, and profiling in CI.

What Success Looks Like (45 seconds)

* We run multiple funding rounds in parallel; voters cast Cardano‑verified votes; historic data is
  publicly auditable; and the stack runs without federated servers or Web2 storage.
* The CEO can summarize it simply: “Catalyst runs on a distributed, verifiable engine; we scale
  rounds in parallel; every result is auditable end‑to‑end.”

Close (15 seconds)

* Hermes is the engine; Athena is the experience.
  This roadmap replaces central points of failure with a distributed foundation, and it brings governance
  features that move beyond today’s proof‑of‑concept.
* We’re ready to begin.

Appendix: Quick Code Pointers (not for reading aloud)

* Engine/WASM: `hermes/bin/src/wasm/module.rs`, `hermes/bin/src/wasm/engine.rs`
* Eventing/Parallelism: `hermes/bin/src/event/queue.rs`, `hermes/bin/src/pool.rs`
* Runtime Extensions: `hermes/bin/src/runtime_extensions/hermes/*`, `docs/src/architecture/08_concepts/runtime_extensions.md`
* IPFS + Doc Sync: `hermes/bin/src/ipfs/mod.rs`, `docs/src/architecture/08_concepts/document_sync/*`,
  `wasm/wasi/wit/deps/hermes-doc-sync/*`
* HTTP Gateway: `hermes/bin/src/runtime_extensions/hermes/http_gateway/*`, `docs/src/architecture/08_concepts/http_gateway.md`
* Athena Modules: `hermes/apps/athena/modules/*`
