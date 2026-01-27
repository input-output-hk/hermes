---
icon: material/horse-variant-fast
---

# Quick Start

This walkthrough uses the root `justfile` to build and run the Athena reference app.

## Prereqs

* Rust and rustup (for local builds).
* `just` command runner.
* Optional: Earthly plus Docker/Podman for containerized builds.

## Fast local flow (recommended)

```sh
just check-local-build
just build-run-dev-fastest
```

## Containerized flow (matches CI)

```sh
just build-run-dev
```

## Verify the gateway

```sh
curl -H "Host: app.hermes.local" http://localhost:5000/api/gateway/v1/rbac/registration
```

## Clean state

Hermes persists per-app state in `~/.hermes/*.hfs`.

```sh
just clean-hfs
```
