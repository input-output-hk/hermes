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

3. **Build and run:**

   ```bash
   # First time setup (run once)
   just check-local-build
   
   # Choose your build approach:
   just build-run-dev-fastest  # 🚀 Daily dev (local, fastest)
   just build-run-dev          # 🐳 Team consistency (containerized)
   just build-run-all          # 📦 Production (full assets)
   ```

## Build System

This project uses [Just](https://github.com/casey/just) with two build approaches:

| Approach | When to Use | Requirements |
|----------|-------------|-------------|
| 🚀 **Local** | Daily development, rapid iteration | Local Rust + `wasm32-wasip2` |
| 🐳 **Containerized** | Team consistency, CI/CD, final testing | Docker/Podman + Earthly |

**All detailed documentation is in the `justfile`.** Run `just --list` to see all commands.

## Key Commands

### Build Commands

**Main workflows:**
* `just build-run-dev-fastest` - 🚀 **Daily development** (local builds, fastest)
* `just build-run-dev` - 🐳 **Team consistency** (containerized, matches CI)  
* `just build-run-all` - 📦 **Production** (full assets, deployments)

**Quick rebuilds:**
* `just dev-athena-fast` - WASM only (development)
* `just dev-athena` - WASM only (production)

**Setup:**
* `just check-local-build` - Verify local Rust (run once)

### Other Commands

* `just status` - Show build status
* `just clean-hfs` - Clean application state  
* `just --list` - See all available commands

For detailed help: `just --show <command>`

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
