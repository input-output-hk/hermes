---
icon: material/gate
---

# HTTP Gateway

The HTTP gateway exposes a single HTTP endpoint per node and routes requests to applications and modules.

Routing model

* Application selection via hostname: `<app>.<domain>` determines the target application (e.g., `athena.hermes.local`).
* Endpoint subscriptions: Modules can be associated with HTTP methods, path regexes, and content-types;
  incoming requests are matched and routed accordingly.
* Fallback `/api` path: Requests under `/api` are treated as WebAssembly API calls; others are treated as static file requests.

Static asset serving

* Files under `srv/www` in the application package are served directly from the VFS.
* Safe path normalization and validation prevent traversal.

Module API requests

* Requests that match subscriptions (or `/api`) are wrapped into events and dispatched to the target module via the event queue.
* Modules execute with a fresh runtime context; the result is returned to the client.

Configuration

* Endpoint subscriptions can be provided by an embedded JSON file and loaded on gateway init.

References

* Source: `hermes/bin/src/runtime_extensions/hermes/http_gateway/`
* Router: `routing.rs`
