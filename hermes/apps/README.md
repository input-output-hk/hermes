# Hermes Apps

This directory contains Hermes applications and their build configuration.

## Available Commands

Use `just` to run the following commands:

### Build Commands

- `just build-hermes` - Build the Hermes binary in release mode
- `just build-athena` - Build and package the Athena HTTP proxy component
- `just build-all` - Build Hermes binary and Athena component

### Run Commands

- `just run-athena` - Run the Athena application using the release binary
- `just run-all` - Build everything and run the Athena application

## Quick Start

1. **Build and run everything:**
   ```bash
   just run-all
   ```

2. **Build only (without running):**
   ```bash
   just build-all
   ```

3. **Run Athena (requires prior build):**
   ```bash
   just run-athena
   ```

## Requirements

- Rust toolchain
- Earthly (for building WASM components)
- Just command runner
- GITHUB_TOKEN environment variable (for Earthly builds)

## Applications

### Athena

HTTP proxy application built on the Hermes engine.

- **Location**: `athena/`
- **Manifest**: `athena/manifest_app.json`
- **Module Manifest**: `athena/manifest_module.json`
- **Binary**: `athena/app.happ` (generated after build)

The Athena application runs with the `--untrusted` flag for security isolation.