---
icon: material/gate
---

# HTTP Gateway

The HTTP gateway exposes a single HTTP endpoint per node and routes requests to applications and modules.

Routing model

* Application selection via hostname: `<app>.<hostname>` determines the target application (e.g., `athena.hermes.local`).
  The gateway currently accepts `hermes.local` and `localhost` hostnames by default.
* Endpoint subscriptions: Modules can be associated with HTTP methods, path regexes, and content-types;
  incoming requests are matched and routed accordingly.
* Fallback `/api` path: Requests under `/api` are treated as WebAssembly API calls; others are treated as static file requests.

Static asset serving

* Files under `srv/www` in the application package are mounted into the VFS at `www/`
  and served directly from there.
* Safe path normalization and validation prevent traversal.

Module API requests

* Requests that match subscriptions (or `/api`) are wrapped into events and dispatched to the target module via the event queue.
* Modules execute with a fresh runtime context; the result is returned to the client.

Configuration

* Endpoint subscriptions are loaded from the embedded `config/endpoints.json`.
* `HERMES_HTTP_PORT` controls the listening port (default: 5000).
* `HERMES_ACTIVATE_AUTH` enables the auth gateway for protected endpoints.

References

* Source: `hermes/bin/src/runtime_extensions/hermes/http_gateway/`
* Router: `routing.rs`
