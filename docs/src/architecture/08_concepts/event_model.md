---
icon: material/timeline
---

# Event Model and Concurrency

Hermes translates external stimuli (HTTP, IPFS pub/sub, DHT updates, chain follower events, cron)
into events that are enqueued and executed by modules.

Queue and dispatch

* Singleton MPSC event queue accepts `HermesEvent`s; each carries a payload and targeting
  (all apps, specific apps, and optionally specific modules).
* Reactor resolves target applications/modules and dispatches events via a worker pool.
* The queue is FIFO, but dispatched work can overlap; strict per-module ordering requires
  disabling parallel execution.

Execution model

* Modules are instantiated with imports pre-linked to reduce per-call overhead.
* Each event execution runs with a new runtime context (immutable module state approach).
* Failures are logged and isolated to the module/app; the reactor continues.

Dependency tracking (extension-level)

* Certain sources (e.g., HTTP request/response, blockchain blocks and derived transactions) may require sequential
  processing where a parent event must complete before children.
* Extensions can enforce ordering with their own coordination mechanisms (e.g., the HTTP gateway uses a per-request
  MPSC channel to block the caller until a module replies or the channel closes).
* There is no shared, generic dependency-tracking layer in the core event queue; ordering depends on extension logic
  and whether parallel execution is enabled.

References

* `hermes/bin/src/event/mod.rs`, `hermes/bin/src/event/queue.rs`
* `hermes/bin/src/app.rs`, `hermes/bin/src/reactor.rs`
