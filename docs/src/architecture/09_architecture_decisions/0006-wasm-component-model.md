---
    title: 0006 WASM Component Model with Wasmtime and WIT
    adr:
        author: Steven Johnson <steven.johnson@iohk.io>
        created: 21-Oct-2025
        status:  accepted
        extends:
            - 0001-arch-std
            - 0002-adr
    tags:
        - arc42
        - ADR
---

## Context

Hermes needs a portable execution model for application logic across languages.
Raw WASM exposes only primitive types and places memory management burdens on each module
(custom alloc/free exports, manual marshalling).

We require:

* Language‑neutral interface definitions
* Rich types (records, variants, lists, strings) passed across the boundary safely
* Generated bindings for host functions and component exports

Alternatives considered

* Raw WASM with hand‑rolled ABI: brittle, error‑prone memory handling, slow team velocity
* WASI (legacy `witx`): not sufficient alone; we need component exports and richer IDL

## Decision

Adopt the WASM Component Model with Wasmtime and WIT for all host capability interfaces and module exports.

## Consequences

Positive

* Strong, typed boundary with generated bindings; reduced boilerplate and fewer ABI bugs
* Clear contract for runtime extensions to expose host APIs
* Pre‑link imports into `wasmtime::InstancePre` for efficient instantiation

Trade‑offs and risks

* Ecosystem and tools are still evolving; track upstream changes
* Bindings regeneration becomes a required build step

## Implementation

* `hermes/bin/src/runtime_extensions/bindings` uses `wasmtime::component::bindgen` over WIT files
* `hermes/bin/src/wasm/*` manages engine config, module instantiation, runtime context, and event entrypoints

## References

* Concepts: 05_building_block_view/hermes_engine.md
* Runtime Extensions: 08_concepts/runtime_extensions.md

---
