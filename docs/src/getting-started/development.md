---
icon: material/hammer-wrench
---

# Development

This page describes the day-to-day workflows for Hermes engine and Athena modules.
Most commands are intended to run from the repo root.

## Prereqs

* Rust (stable) and rustup.
* `just` command runner.
* Optional: Earthly plus Docker/Podman for containerized builds.
* WASM target for local module builds: `rustup target add wasm32-wasip2`.

## Tooling map

* Root `justfile`: end-to-end Athena build/run flows.
* `hermes/Justfile`: engine linting and CI parity checks.
* `hermes/Earthfile`: engine builds and CI targets.
* `wasm/wasi/Earthfile`: WIT docs and binding generation helpers.

## Justfile patterns

The root `justfile` offers two styles of workflows:

* Local builds (fast): `just check-local-build` then `just build-run-dev-fastest`.
* Containerized builds (CI-like): `just build-run-dev` or `just build-run-all`.

For fast iteration on modules only:

* `just dev-athena-fast` (dev assets) or `just dev-athena` (full assets).

Engine-only rebuilds:

* `just get-local-hermes-fast` (local Rust).
* `just get-local-hermes` (Earthly, containerized).

## Common workflows

Engine-only changes

```sh
just get-local-hermes-fast
```

Module-only changes (local)

```sh
just get-local-athena-fast
just dev-athena-fast
```

Full app rebuild

```sh
just build-run-dev-fastest
```

Containerized builds (CI-like)

```sh
just build-run-dev
```

## Packaging with the Hermes CLI

Module manifests live under each module:

* `hermes/apps/athena/modules/<module>/lib/manifest_module.json`

App manifest:

* `hermes/apps/athena/manifest_app.json`

Example commands:

```sh
target/release/hermes module package hermes/apps/athena/modules/http-proxy/lib/manifest_module.json
target/release/hermes app package hermes/apps/athena/manifest_app.json
```

## WIT interfaces and bindings

* WIT sources: `wasm/wasi/wit`.
* Generated bindings: `hermes/bindings.rs` (see `earthly ./hermes+bindings-expand`).

## Handy commands

* `just --show <task>` explains a task from the root `justfile`.
* `just status` shows build artifacts and sizes.
