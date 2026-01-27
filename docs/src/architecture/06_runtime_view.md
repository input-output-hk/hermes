---
icon: material/run-fast
---

# Runtime View

<!-- See: https://docs.arc42.org/section-6/ -->

## Application bootstrap

* CLI `run` parses arguments, loads certificates, validates application package (metadata, structure, signatures).
* Engine initializes reactor and event queue, starts embedded IPFS node, bootstraps VFS from package,
  and (unless disabled) initializes the global worker pool for parallel event execution.
* Application is registered, its modules are instantiated (pre-linked), and `init` is invoked if exported.

## HTTP request handling (Athena example)

* Client sends request to `https://<app>.<domain>/...`.
* HTTP gateway resolves `<app>`, matches endpoint subscription or `/api` route, and constructs an HTTP event.
* Event queued to the reactor; target module is located; module executes with a fresh runtime context.
* Response returned to the gateway and to the client; static assets are served directly from VFS when applicable.

## IPFS pub/sub message (Athena example)

* A module subscribes to a topic via the host API; the engine ensures a topic stream exists and registers the subscription.
* When a pub/sub message arrives, the runtime performs basic content validation and
  emits an event to the subscribing application(s)/module(s).
* Modules process the message and may publish responses or update DHT entries.

## Chain follower event (Athena example)

* A runtime extension ingests blockchain data (from tip, genesis, or a specific point) and normalizes it to events
  (e.g., new block, transaction).
* Dependency tracking is handled by extensions as needed (e.g., per-request response channels or explicit sequencing);
  ordering depends on extension logic and the parallel execution mode.
* Modules consume block/tx events and may emit further events or perform reads/writes via VFS/IPFS.

## Graceful shutdown

* CLI requests shutdown or timeout is reached; event queue receives a `Break` control message.
* Runtime extensions cancel background tasks (e.g., chain sync); if parallel execution is enabled,
  the worker pool waits for in-flight tasks to finish before exit.
