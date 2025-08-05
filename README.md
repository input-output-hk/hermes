# Hermes

<!-- markdownlint-disable MD029 -->

# Hermes

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
   ```bash
   just build-run-all
   ```

## All Documentation is in the Justfile

This project uses [Just](https://github.com/casey/just) for build automation.
**All build instructions, prerequisites, configuration options, development workflows,
and detailed documentation are contained in the `justfile`.**

Run `just --list` to see all available commands with their descriptions,
or `just --show <command>` to see detailed documentation for any specific command.

## Key Commands

- `just build-run-all` - Complete workflow (recommended for first-time users)
- `just status` - Show current build status and configuration
- `just --help` - Just command help

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
