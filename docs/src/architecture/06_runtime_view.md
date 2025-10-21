---
icon: material/run-fast
---

# Runtime View

<!-- See: https://docs.arc42.org/section-6/ -->

## Application bootstrap

- CLI `run` parses arguments, loads certificates, validates application package (metadata, structure, signatures).
- Engine initializes reactor and event queue, starts embedded IPFS node, bootstraps VFS from package.
- Application is registered, its modules are instantiated (pre-linked), and `init` is invoked if exported.

## HTTP request handling

- Client sends request to `https://<app>.<domain>/...`.
- HTTP gateway resolves `<app>`, matches endpoint subscription or `/api` route, and constructs an HTTP event.
- Event queued to the reactor; target module is located; module executes with a fresh runtime context.
- Response returned to the gateway and to the client; static assets are served directly from VFS when applicable.

## IPFS pub/sub message

- A module subscribes to a topic via the host API; the engine ensures a topic stream exists and registers the subscription.
- When a pub/sub message arrives, the runtime validates it (per-topic/content strategy) and emits an event to the subscribing application(s)/module(s).
- Modules process the message and may publish responses or update DHT entries.

## Chain follower event (example)

- A runtime extension ingests blockchain data (from tip, genesis, or a specific point) and normalizes it to events (e.g., new block, transaction).
- Dependency tracking ensures source ordering while allowing unrelated sources to proceed.
- Modules consume block/tx events and may emit further events or perform reads/writes via VFS/IPFS.

## Graceful shutdown

- CLI requests shutdown or timeout is reached; event queue receives a `Break` control message.
- Reactor drains in-flight work; worker pool terminates; runtime extensions finalize; process exits with code.
