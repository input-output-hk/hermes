# Hermes Apps

This directory contains Hermes applications and their build configuration. These applications demonstrate how to build modular, WASM-based services using the Hermes engine.

## Architecture Overview

Hermes applications are composed of:
- **WASM Modules**: Individual components compiled to WebAssembly (WASI-P2)
- **Application Manifests**: JSON configurations that define how modules are loaded and configured
- **Module Manifests**: Metadata describing individual WASM components
- **Configuration Files**: Runtime settings and schemas for validation

## Available Commands

Use `just` to run the following commands from this directory:

### Build Commands

- **`just build-hermes`** - Build the Hermes binary in release mode
  - Compiles the core Hermes engine located in `../`
  - Output: `../target/release/hermes`
  - Required for packaging and running applications

- **`just build-athena`** - Build and package the Athena HTTP proxy component
  - Generates Rust bindings from WIT interfaces
  - Compiles the HTTP proxy module to WASM using Earthly
  - Packages the module using the Hermes CLI
  - Creates the final application package
  - Output: `athena/app.happ`

- **`just build-all`** - Build Hermes binary and Athena component
  - Runs both `build-hermes` and `build-athena` in sequence
  - Recommended for clean builds

### Run Commands

- **`just run-athena`** - Run the Athena application using the release binary
  - Requires prior build (`just build-all` or individual build commands)
  - Runs with `--untrusted` flag for security isolation
  - Uses the packaged application at `athena/app.happ`

- **`just run-all`** - Build everything and run the Athena application
  - Complete workflow: build → package → run
  - Equivalent to `just build-all && just run-athena`

## Quick Start

### Prerequisites

Before building, ensure you have:

1. **Earthly** for containerized builds:
   ```bash
   # Install from https://earthly.dev/get-earthly
   curl -fsSL https://earthly.dev/install.sh | sh
   ```

2. **Just command runner**:
   ```bash
   # Install via cargo (if you have Rust) or package manager
   cargo install just
   # Or via package manager (Ubuntu/Debian)
   sudo apt install just
   # Or download binary from https://github.com/casey/just/releases
   ```

3. **GitHub Token** (for Earthly builds):
   ```bash
   export GITHUB_TOKEN=your_github_token_here
   ```

**That's it!** No need to install Rust locally - Earthly handles all compilation in containerized environments with pre-configured toolchains.

### Building and Running

1. **Build and run everything (recommended):**
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

### Development Workflow

For development, you can build components individually:

```bash
# Build just the Hermes engine
just build-hermes

# Build just the Athena WASM module
just build-athena

# Run with existing build
just run-athena
```

## Applications

### Athena - HTTP Proxy Service

A secure HTTP/HTTPS redirect service with configurable validation and routing policies.

**Architecture:**
- **Location**: `athena/`
- **Module Source**: `athena/modules/http-proxy/`
- **Application Manifest**: `athena/manifest_app.json`
- **Module Manifest**: `athena/modules/http-proxy/lib/manifest_module.json`
- **WASM Binary**: `athena/modules/http-proxy/lib/http_proxy.wasm` (generated)
- **Application Package**: `athena/app.happ` (generated)

**Key Features:**
- HTTP request/response handling
- Configurable routing policies
- Security validation
- WASM-based isolation

**Configuration Files:**
- `config.schema.json` - Runtime configuration schema
- `settings.schema.json` - Application settings schema
- `metadata.json` - Application metadata and licensing

**Security:**
- Runs with `--untrusted` flag for maximum isolation
- WASM sandbox provides additional security boundaries
- Configurable validation policies

## Build Process Deep Dive

### 1. Binding Generation
```bash
earthly +gen-bindings
```
- Generates Rust bindings from WIT (WebAssembly Interface Types)
- Creates `hermes.rs` with all necessary interfaces
- Required before compiling WASM modules

### 2. WASM Compilation
```bash
earthly +build-http-proxy
```
- Compiles Rust code to `wasm32-wasip2` target
- Uses optimized release profile (`opt-level = "z"`, `lto = true`)
- Produces highly optimized WASM binary

### 3. Module Packaging
```bash
hermes module package manifest_module.json
```
- Validates module manifest against schema
- Bundles WASM binary with configuration files
- Creates distributable module package

### 4. Application Packaging
```bash
hermes app package manifest_app.json
```
- Combines modules into complete application
- Validates application manifest
- Creates final `.happ` (Hermes App Package) file

## Troubleshooting

### Common Issues

**Build fails with "No Earthfile found":**
- Ensure you're running commands from the `hermes/apps/` directory
- Check that `GITHUB_TOKEN` environment variable is set

**WASM compilation errors:**
- Verify `wasm32-wasip2` target is installed: `rustup target list --installed`
- Check Rust version compatibility

**Permission denied errors:**
- Ensure Earthly daemon is running and accessible
- Check Docker/Podman permissions

**Missing hermes binary:**
- Run `just build-hermes` first
- Check that `../target/release/hermes` exists

### Debug Mode

For debugging builds, you can run Earthly commands directly:

```bash
cd athena/modules
earthly +gen-bindings --no-cache
earthly +build-http-proxy --no-cache
```

## Adding New Applications

To add a new application:

1. Create application directory structure
2. Implement WASM modules using the Hermes APIs
3. Create manifest files following the schema
4. Add build targets to the justfile
5. Test with `--untrusted` flag for security

See the Athena implementation as a reference example.

## File Structure

```
hermes/apps/
├── justfile                    # Build automation
├── README.md                   # This file
└── athena/                     # Athena HTTP proxy app
    ├── manifest_app.json       # Application manifest
    ├── app.happ               # Generated app package
    └── modules/
        ├── Earthfile          # Build configuration
        └── http-proxy/
            ├── src/           # Rust source code
            ├── Cargo.toml     # Rust dependencies
            └── lib/           # Module resources
                ├── manifest_module.json
                ├── metadata.json
                ├── config.schema.json
                ├── settings.schema.json
                └── http_proxy.wasm  # Generated WASM
```