# Hermes

<!-- markdownlint-disable MD029 -->

Hermes is a high-performance WebAssembly (WASM) application engine that provides secure,
sandboxed execution of modular applications.

## Quick Start

0. **Github token**:

   ## GitHub Token Setup

   * Go to [github.com/settings/tokens](https://github.com/settings/tokens)
   * Generate new classic token with + permissions
   * Add to .secret file.

1. **Install Just command runner**:

   ```bash
   cargo install just
   # Or: sudo apt install just
   # Or: brew install just
   ```

2. **See all available commands and documentation**:

   ```bash
   just --list
   ```

3. **Build and run everything**:

   **🚀 Fastest (local builds, recommended for daily dev):**

   ```bash
   just check-local-build      # Run once to verify setup
   just build-run-dev-fastest  # faster than containerized builds
   ```

   **🐳 Reliable (containerized, matches CI):**

   ```bash
   just build-run-dev         # Safe fallback, team consistency
   ```

   **📦 Production (full assets, slow):**

   ```bash
   just build-run-all         # Complete with all web assets
   ```

## Build System Overview

This project uses [Just](https://github.com/casey/just) for build automation with two approaches:

**🚀 Local Builds** - Use your local Rust toolchain directly:
- **3-5x faster** than containerized builds
- Perfect for daily development iteration
- Requires local Rust with `wasm32-wasip2` target
- Use when: rapid prototyping, personal productivity

**🐳 Containerized Builds** - Use Earthly containers:
- **Consistent** across all environments
- Matches CI/CD pipeline exactly
- No local setup required
- Use when: team consistency, final testing, CI/CD

**All build instructions, prerequisites, configuration options, development workflows,
and detailed documentation are contained in the `justfile`.**

Run `just --list` to see all available commands with their descriptions,
or `just --show <command>` to see detailed documentation for any specific command.

## Key Commands

### Build & Run

**🚀 Local Builds (fastest):**
* `just check-local-build` - Verify local Rust setup (run once)
* `just build-run-dev-fastest` - **Fastest dev build** (uses local Rust)
* `just get-local-hermes-fast` - Build just Hermes locally
* `just get-local-athena-fast` - Build just WASM modules locally

**🐳 Containerized Builds (reliable, matches CI):**
* `just build-run-dev` - **Development build** (containerized)
* `just build-run-all` - **Production build** (includes all web assets)
* `just dev-athena-fast` - Quick WASM rebuild for development iteration

### Utilities

* `just status` - Show current build status and configuration
* `just clean-hfs` - Clean up previous application state
* `just --help` - Just command help

For everything else - architecture, prerequisites, configuration,
troubleshooting, development workflows - see the justfile documentation
via `just --list`.

## Development

For development guidelines, tooling information, and best practices, see DEVELOPMENT.md.

## Contributing

We welcome contributions from the community!
Please read our [CONTRIBUTING](CONTRIBUTING.md) for guidelines on how to contribute.

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or [MIT license](LICENSE-MIT) at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in this crate by you, as defined in the Apache-2.0 license, shall
be dual licensed as above, without any additional terms or conditions.
