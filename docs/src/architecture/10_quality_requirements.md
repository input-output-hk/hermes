---
icon: material/quality-high
---

# Quality Requirements

<!-- See: https://docs.arc42.org/section-10/ -->

## Quality tree

* Modularity and Extensibility
    * Clear separation between engine, runtime extensions, and apps
    * WIT-based interfaces and generated bindings
* Security and Integrity
    * Immutable, signed packages; constrained host APIs; certificate trust
    * Topic/message signature validation for P2P (planned; current checks are basic)
* Performance and Scalability
    * Pre-linked WASM instances; per-core worker pool; backpressure via queue
    * Static file serving from VFS
* Reliability and Availability
    * FIFO event queue; optional serial execution for ordering; isolation on failure; graceful shutdown
* Observability
    * Structured logging/tracing; build info emission; tunable verbosity
* Developer Experience
    * CLI tooling for build/sign/run; standard Web/Flutter frontends; sample modules

## Quality scenarios

1) High-traffic API burst
   * HTTP gateway classifies and enqueues requests; worker pool scales over cores; non-API static assets served directly.

2) Invalid package provided
   * Signature and schema validation fail; app not loaded; error surfaced with actionable diagnostics.

3) P2P message flood from misbehaving peer
   * Basic validation drops invalids; peer can be evicted; engine continues to serve other traffic.

4) Module crash during event handling
   * Error logged; event processing for that module fails fast; other modules/apps unaffected.

5) Node restart
   * IPFS repo reused; app package revalidated and reloaded; idempotent init; HTTP gateway rebinds.

## Quality Tree

## Quality Scenarios
