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
* Per-target ordering is preserved by enqueue order; unrelated sources can proceed concurrently.

Execution model

* Modules are instantiated with imports pre-linked to reduce per-call overhead.
* Each event execution runs with a new runtime context (immutable module state approach).
* Failures are logged and isolated to the module/app; the reactor continues.

Dependency tracking (Athena MVP intent)

* Certain sources (e.g., blockchain blocks and derived transactions) may require sequential processing
  where a parent event must complete before children.
* A simple interlocking pattern using per-source queues can enforce ordering while allowing unrelated sources to progress.

References

* `hermes/bin/src/event/mod.rs`, `hermes/bin/src/event/queue.rs`
* `hermes/bin/src/app.rs`, `hermes/bin/src/reactor.rs`
