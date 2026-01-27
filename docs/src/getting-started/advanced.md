---
icon: material/rotate-orbit
---

# Advanced

This page covers Earthly targets, test preparation steps, and runtime knobs.

## Earthly targets

Engine and CI parity:

```sh
earthly ./hermes+check
earthly ./hermes+build
earthly ./hermes+build-athena
```

WIT docs and bindings:

```sh
earthly ./wasm/wasi+build
earthly ./wasm/wasi+build-rust-bindings
```

Docs and spelling checks:

```sh
earthly +markdown-check-fix
earthly +check-spelling
```

## Integration tests and benches

Build a WASM component before benches:

```sh
earthly ./wasm/c+save-local
cargo bench --features bench
```

Build integration test components before tests:

```sh
earthly ./wasm+save-c-integration-test-local
cargo test
```

## Runtime flags and environment

CLI flags:

* `hermes run --untrusted` skips signature verification (dev only).
* `--no-parallel-event-execution` forces serial event handling.
* `--serialize-sqlite` serializes SQLite access.
* `--timeout-ms` sets a process timeout.

Environment variables:

* `HERMES_LOG_LEVEL` controls log verbosity.
* `HERMES_HTTP_PORT` and `HERMES_ACTIVATE_AUTH` configure the HTTP gateway.
* `REDIRECT_ALLOWED_HOSTS` and `REDIRECT_ALLOWED_PATH_PREFIXES` control redirect allowlists.
* `IPFS_BOOTSTRAP_PEERS`, `IPFS_LISTEN_PORT`, `IPFS_ANNOUNCE_ADDRESS`,
  `IPFS_RETRY_INTERVAL_SECS`, `IPFS_MAX_RETRIES` tune IPFS behavior.
