# Hermes Applications

This directory contains production Hermes applications and their build configuration. These applications demonstrate real-world usage of the Hermes WASM application engine.

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

### Run Commands

- **`just run-athena`** - Run the Athena application using the release binary
  - Requires prior build (`just build-hermes` and `just build-athena`)
  - Runs with `--untrusted` flag for security isolation
  - Uses the packaged application at `athena/app.happ`

- **`just run-all`** - Build everything and run the Athena application
  - Complete workflow: build → package → run
  - Equivalent to `just build-hermes && just build-athena && just run-athena`

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

**That's it!** No need to install Rust locally - Earthly handles all compilation in containerized environments with pre-configured toolchains.

### Building and Running

1. **🚀 Complete workflow - Build and run everything (START HERE):**
   ```bash
   just build-run-all
   ```
   **This single command does EVERYTHING you need:**
   - Cleans up any previous state (`clean-hfs`)
   - Builds the Hermes engine (`build-hermes`) 
   - Builds and packages the Athena application (`build-athena`)
   - Runs the application (`run-athena`)
   
   **Use this command for your first build and whenever you want a clean, complete rebuild.**

2. **Individual commands (optional - only if you need granular control):**
   ```bash
   # Build just the Hermes engine
   just build-hermes
   
   # For development, you can build components individually:
   # Build just the Athena WASM modules and package
   just build-athena
   
   # Run Athena (requires prior build)
   just run-athena
   
   # Clean up state files
   just clean-hfs
   ```

### Development Workflow

For most development work, simply use:
```bash
just build-run-all
```

You can also override these values for the complete build and run process:
```bash
REDIRECT_ALLOWED_HOSTS=example.com REDIRECT_ALLOWED_PATH_PREFIXES=/api just build-run-all
```
This builds the entire application with custom redirect security settings and runs it in one command. 
If no values are provided, it uses the defaults (i.e., for redirect allowed hosts and for redirect allowed path prefixes). `catfact.ninja``/fact`

## Testing the System

Once you have the application running with `just build-run-all`, you can test the HTTP proxy functionality using these curl commands:

### Direct API Call (for comparison)
```bash
time curl -X 'GET' \
  'https://catfact.ninja/fact' \
  -H 'accept: application/json' \
  -H 'X-CSRF-TOKEN: MXsgfJDha1E362NJkyMYQmjSLUGSjlW9AM4iQQD2'
```

### Through Athena Proxy
```bash
time curl -X GET \
  -H "Content-type: application/json" \
  -H "Accept: application/json" \
  -H "Host: app.hermes.local" \
  "http://0.0.0.0:5000/api/dashboard" | jq -r '.[2] | map([.] | implode) | join("")'
```

The dashboard endpoint routes requests through the WebAssembly proxy component, 
applying security validation and redirect controls before forwarding to external APIs. 
You can experiment with different and values to see how the security configuration affects request routing. 



## Applications

### Athena - HTTP Proxy Service

A secure HTTP/HTTPS redirect service with configurable validation and routing policies.

**Current Status**: Athena currently contains **one module** - the HTTP proxy component. This serves as our initial production application demonstrating the Hermes WASM engine capabilities. **Additional modules will be added in future releases** to expand Athena's functionality.

**Architecture:**
- **Location**: `athena/`
- **Current Module**: `athena/modules/http-proxy/` (smart HTTP proxy)
- **Future Modules**: Additional WASM components will be added here
- **Application Manifest**: `athena/manifest_app.json`
- **Module Manifest**: `athena/modules/http-proxy/lib/manifest_module.json`
- **WASM Binary**: `athena/modules/http-proxy/lib/http_proxy.wasm` (generated)
- **Application Package**: `athena/app.happ` (generated)

**Current HTTP Proxy Module Features:**
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

> **Note**: As we add more modules to Athena, each will have its own directory under `athena/modules/` and will be packaged together into the single `athena/app.happ` application bundle.

## Build Process Deep Dive

### 1. Binding Generation
- Generates Rust bindings from WIT (WebAssembly Interface Types)
- Creates `hermes.rs` with all necessary interfaces
- Required before compiling WASM modules

### 2. WASM Compilation
- Compiles Rust code to `wasm32-wasip2` target
- Uses optimized release profile (`opt-level = "z"`, `lto = true`)
- Produces highly optimized WASM binary

### 3. Module Packaging
- Validates module manifest against schema
- Bundles WASM binary with configuration files
- Creates distributable module package

### 4. Application Packaging
- Combines modules into complete application
- Validates application manifest
- Creates final `.happ` (Hermes App Package) file

## Troubleshooting

### Common Issues

**Build fails with "No Earthfile found":**
- Ensure you're running commands from the `hermes/apps/` directory

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

```hermes/apps/
├── justfile   # Build automation
├── README.md  # This file
└── athena/    # Athena HTTP proxy app
├── manifest_app.json  # Application manifest
├── app.happ # Generated app package
└── modules/
├── Earthfile  # Build configuration
└── http-proxy/
├── src/   # Rust source code
├── Cargo.toml   # Rust dependencies
└── lib/  # Module resources
├── manifest_module.json
├── metadata.json
├── config.schema.json
├── settings.schema.json
└── http_proxy.wasm  # Generated WASM