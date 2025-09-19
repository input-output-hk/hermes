# Hermes Applications Build System
#
# This justfile provides a complete build and deployment workflow for Hermes WASM applications.
# It uses Earthly for containerized builds to ensure consistent, reproducible compilation
# across different development environments.
#
# Prerequisites:
#   - Earthly (https://earthly.dev/get-earthly) - Containerized build system
#   - Docker or Podman - Container runtime for Earthly
#
# Quick Start:
#   just build-run-all    # Complete workflow: build → package → run
#
# Development Workflow:
#   just clean-hfs        # Clean up previous state
#   just get-local-hermes # Build the core engine
#   just get-local-athena     # Build & package WASM modules
#   just run-athena       # Run the application
#
# File Formats:
#   .happ - Hermes Application Package (complete application bundle)
#   .hmod - Hermes Module Package (individual WASM component with manifest)
#   .hfs  - Hermes File System state files (runtime cache and temporary data)
#
# Environment Variables:
#   REDIRECT_ALLOWED_HOSTS - Comma-separated allowed redirect hosts (default: app.dev.projectcatalyst.io)
#   REDIRECT_ALLOWED_PATH_PREFIXES - Allowed path prefixes for redirects (default: /api/gateway/v1/config/frontend)

default:
    @just --list --unsorted

# Build the Hermes binary in release mode using Earthly
#
# This target compiles the core Hermes WASM application engine located in the parent
# directory (../). The engine provides the runtime environment for WASM modules and
# handles HTTP routing, logging, security sandboxing, and module lifecycle management.
#
# Build Process:
#   1. Uses Earthly for containerized compilation (no local Rust toolchain required)
#   2. Compiles with release optimizations (--release flag)
#   3. Outputs binary to ../target/release/hermes
#   4. Makes binary executable and copies to working location
#
# Output: ../target/release/hermes (executable binary)
# Duration: ~2-5 minutes (depending on system and cache state)
# Dependencies: None (self-contained with Earthly)
get-local-hermes:
    #!/usr/bin/env bash
    set -euo pipefail

    echo "🔨 Building Hermes binary with Earthly..."
    echo "📍 Building from: $(realpath ../)"
    echo "🎯 Target: Release binary with optimizations"

    # Build using Earthly's containerized environment
    # This ensures consistent builds regardless of local toolchain
    earthly ./hermes/apps/athena/modules+save-local-hermes

    # Ensure target directory exists for binary placement
    mkdir -p target/release

    # Copy the binary from Earthly output to expected location
    cp hermes/apps/athena/modules/hermes target/release/hermes
    chmod +x target/release/hermes

    echo "✅ Hermes build complete!"
    echo "📦 Binary location: $(realpath target/release/hermes)"
    echo "📏 Binary size: $(ls -lh target/release/hermes | awk '{print $5}')"

# Build and package the Athena HTTP proxy WASM component
#
# This target performs the complete build pipeline for the Athena application:
#   1. Generates Rust bindings from WebAssembly Interface Types (WIT)
#   2. Compiles HTTP proxy module to WASM using wasm32-wasip2 target
#   3. Packages the WASM module with its manifest and configuration (.hmod file)
#   4. Creates the final application package (.happ file)
#
# Build Pipeline:
#   Generate WIT Bindings → Compile to WASM → Package Module (.hmod) → Package Application (.happ)
#
# Components Built:
#   - HTTP Proxy Module: athena/modules/http-proxy/ (Rust → WASM)
#   - Module Package: Individual WASM component with manifest (.hmod format)
#   - Application Package: Complete application bundle ready for deployment (.happ format)
#
# Output Files:
#   - athena/modules/http-proxy/lib/http_proxy.wasm (WASM binary)
#   - athena/app.happ (final application package)
#
# Duration: ~3-7 minutes (WASM compilation + packaging)
# Dependencies: WIT files, Rust source code, manifest files
get-local-athena:
    #!/usr/bin/env bash
    set -euo pipefail

    echo "🔨 Building HTTP proxy WASM component..."
    echo "📍 Module location: athena/modules"
    echo "🎯 Target: wasm32-wasip2 (WebAssembly System Interface Preview 2)"

    # Step 1: Build WASM module using Earthly (local development target)
    # This compiles Rust source to optimized WASM binary and saves locally
    earthly ./hermes/apps/athena/modules+local-build-http-proxy
    earthly ./hermes/apps/athena/modules/rbac-registration+local-build-rbac-registration-indexer
    earthly ./hermes/apps/athena/modules/rbac-registration+local-build-rbac-registration
    echo "✅ WASM compilation complete"

    echo "📦 Packaging module with Hermes CLI..."
    echo "📄 Using manifest: hermes/apps/athena/modules/http-proxy/lib/manifest_module.json"

    # Step 2: Package the WASM module with its configuration into .hmod format
    # The .hmod file contains the WASM binary, manifest, and metadata for the module
    target/release/hermes module package hermes/apps/athena/modules/http-proxy/lib/manifest_module.json
    target/release/hermes module package hermes/apps/athena/modules/rbac-registration-indexer/lib/manifest_module.json
    target/release/hermes module package hermes/apps/athena/modules/rbac-registration/lib/manifest_module.json
    echo "✅ Module packaging complete (.hmod file created)"

    echo "📦 Packaging application bundle..."
    echo "📄 Using manifest: hermes/apps/athena/manifest_app.json"

    # Step 3: Create final application package (.happ file)
    # The .happ file bundles all modules and application-level configuration
    target/release/hermes app package hermes/apps/athena/manifest_app.json
    echo "✅ Application packaging complete (.happ file created)"

    echo "🎉 Build and packaging complete!"
    echo "📦 Application package: hermes/apps/athena/app.happ"
    echo "📏 Package size: $(ls -lh hermes/apps/athena/app.happ | awk '{print $5}' 2>/dev/null || echo 'N/A')"

# Clean up Hermes state files from user directory
#
# Removes .hfs (Hermes File System) files from ~/.hermes/ directory.
# These files contain cached application state, temporary data, and runtime artifacts
# that may need to be cleared between development iterations.
#
# When to use:
#   - Before clean builds to ensure fresh state
#   - When debugging application state issues
#   - After significant configuration changes
#   - When switching between different application versions
#
# Files cleaned: ~/.hermes/*.hfs (Hermes state files)
# Safe to run: Only removes application cache, not source code
clean-hfs:
    @echo "🧹 Cleaning Hermes state files..."
    @if [ -d ~/.hermes ]; then \
        echo "📁 Found ~/.hermes/ directory"; \
        find ~/.hermes -name "*.hfs" -type f -delete 2>/dev/null || true; \
        echo "✅ Cleaned up .hfs files from ~/.hermes/"; \
    else \
        echo "ℹ️  ~/.hermes/ directory does not exist (nothing to clean)"; \
    fi

# Run the Athena application using the Hermes runtime
#
# Executes the packaged Athena application in the Hermes WASM runtime environment.
# The application runs with security isolation (--untrusted flag) to demonstrate
# secure execution of WebAssembly components.
#
# Runtime Configuration:
#   - Security: --untrusted flag enables maximum sandboxing
#   - Package: Uses hermes/apps/athena/app.happ (must be built first)
#   - HTTP Server: Typically runs on localhost:5000 (configurable in manifest)
#
# Environment Variables (configurable security policies):
#   REDIRECT_ALLOWED_HOSTS: Comma-separated list of allowed redirect hosts
#     Default: "app.dev.projectcatalyst.io"
#     Example: "api.example.com,service.internal.com"
#
#   REDIRECT_ALLOWED_PATH_PREFIXES: Path prefixes allowed for redirects
#     Default: "/api/gateway/v1/config/frontend,/api/gateway/v1/cardano/assets,/api/gateway/v1/rbac/registration"
#     Example: "/api,/public,/webhooks"
#
# Prerequisites:
#   - Hermes binary must exist (run `just get-local-hermes`)
#   - Application package must exist (run `just get-local-athena`)
#
# Testing the Service:
#   Once running, test with: curl -H "Host: app.hermes.local" http://localhost:5000/api/gateway/v1/rbac/registration
run-athena:
    #!/usr/bin/env bash
    set -euo pipefail

    echo "🚀 Running Athena application..."
    echo "📦 Package: hermes/apps/athena/app.happ"
    echo "🔒 Security: Running with --untrusted flag (maximum isolation)"

    # Validate prerequisites
    if [ ! -f "target/release/hermes" ]; then
        echo "❌ Error: Hermes binary not found. Run 'just get-local-hermes' first."
        exit 1
    fi

    if [ ! -f "hermes/apps/athena/app.happ" ]; then
        echo "❌ Error: Application package not found. Run 'just get-local-athena' first."
        exit 1
    fi

    # Set up security configuration with defaults
    export REDIRECT_ALLOWED_PATH_PREFIXES="${REDIRECT_ALLOWED_PATH_PREFIXES:-/api/gateway/v1/config/frontend,/api/gateway/v1/cardano/assets,/api/gateway/v1/rbac/registration}"
    export REDIRECT_ALLOWED_HOSTS="${REDIRECT_ALLOWED_HOSTS:-app.dev.projectcatalyst.io}"

    echo "🛡️  Security Configuration:"
    echo "   Allowed Hosts: $REDIRECT_ALLOWED_HOSTS"
    echo "   Allowed Path Prefixes: $REDIRECT_ALLOWED_PATH_PREFIXES"
    echo ""
    echo "🌐 Starting HTTP server..."
    echo "💡 Test with: curl -H 'Host: app.hermes.local' http://localhost:5000/api/gateway/v1/rbac/registration"
    echo "🛑 Press Ctrl+C to stop"
    echo ""

    # Execute the application with security sandboxing
    # HERMES_LOG_LEVEL="debug" 
    target/release/hermes run --untrusted hermes/apps/athena/app.happ

# Complete build and run workflow - recommended for most use cases
#
# This is the primary command for development and demonstration. It performs
# the complete workflow from clean state to running application:
#
# Workflow Steps:
#   1. clean-hfs      - Clear previous application state
#   2. get-local-hermes  - Compile the Hermes runtime engine
#   3. get-local-athena   - Build and package WASM modules
#   4. run-athena     - Launch the application
#
# When to use:
#   ✅ First-time setup and builds
#   ✅ Clean rebuilds after significant changes
#   ✅ Demonstration and testing scenarios
#   ✅ When unsure about current build state
#
# Duration: ~5-12 minutes total (depending on system and cache state)
#
# Alternative for incremental development:
#   If only changing WASM module code: just get-local-athena && just run-athena
#   If only changing engine code: just get-local-hermes && just run-athena
#
# Environment Variables: Same as run-athena (see above)
# Example with custom config: REDIRECT_ALLOWED_HOSTS=example.com just build-run-all
build-run-all: clean-hfs get-local-hermes get-local-athena clean-www run-athena

# Development helper: Quick rebuild of just the WASM components
#
# Use this when you're iterating on the HTTP proxy module code and don't need
# to rebuild the entire Hermes engine. Faster than build-run-all for development.
#
# Workflow: Build WASM → Package → Run
# Duration: ~3-5 minutes (skips Hermes engine compilation)
# When to use: Iterating on athena/modules/http-proxy/src/ changes
dev-athena: get-local-athena run-athena

# Show current build status and file information
#
# Displays information about current build artifacts, their sizes, and timestamps.
# Useful for debugging build issues or checking what needs to be rebuilt.
status:
    #!/usr/bin/env bash
    echo "📊 Hermes Applications Build Status"
    echo "=================================="
    echo ""

    echo "🔧 Hermes Engine:"
    if [ -f "../target/release/hermes" ]; then
        echo "   ✅ Binary: $(ls -lh ../target/release/hermes | awk '{print $5 " " $6 " " $7 " " $8}')"
    else
        echo "   ❌ Binary: Not found (run 'just get-local-hermes')"
    fi
    echo ""

    echo "📦 Athena Application:"
    if [ -f "athena/app.happ" ]; then
        echo "   ✅ Package: $(ls -lh athena/app.happ | awk '{print $5 " " $6 " " $7 " " $8}')"
    else
        echo "   ❌ Package: Not found (run 'just get-local-athena')"
    fi

    if [ -f "athena/modules/http-proxy/lib/http_proxy.wasm" ]; then
        echo "   ✅ WASM Module: $(ls -lh athena/modules/http-proxy/lib/http_proxy.wasm | awk '{print $5 " " $6 " " $7 " " $8}')"
    else
        echo "   ❌ WASM Module: Not found"
    fi
    echo ""

    echo "🛡️  Current Security Config:"
    echo "   Allowed Hosts: ${REDIRECT_ALLOWED_HOSTS:-app.dev.projectcatalyst.io (default)}"
    echo "   Allowed Paths: ${REDIRECT_ALLOWED_PATH_PREFIXES:-app.dev.projectcatalyst.io (default)}"
    echo ""

    echo "💡 Quick Commands:"
    echo "   just build-run-all    # Complete rebuild and run"
    echo "   just dev-athena       # Quick WASM rebuild and run"
    echo "   just clean-hfs        # Clear application state"

# Fix and Check Markdown files
check-markdown:
    earthly +markdown-check-fix

# Check Spelling
check-spelling:
    earthly +clean-spelling-list
    earthly +check-spelling

# Pre Push Checks - intended to be run by a git pre-push hook.
pre-push: check-markdown check-spelling
    just hermes/pre-push

# Clean up the www directory from http-proxy module after packaging
#
# Removes the www directory that gets created in the http-proxy module during
# the application packaging process. This directory is located at:
# apps/modules/http-proxy/lib/www
#
# When to use:
#   - After successful packaging to clean up intermediate files
#   - Before clean builds to ensure no stale www content
#   - As part of development iteration to reset web assets
clean-www:
    @echo "🧹 Cleaning up http-proxy www directory..."
    @if [ -d "apps/modules/http-proxy/lib/www" ]; then \
        echo "📁 Found apps/modules/http-proxy/lib/www/ directory"; \
        rm -rf apps/modules/http-proxy/lib/www/; \
        echo "✅ Removed apps/modules/http-proxy/lib/www/ directory"; \
    else \
        echo "ℹ️  apps/modules/http-proxy/lib/www/ directory does not exist (nothing to clean)"; \
    fi

# Enhanced cleanup that includes HFS files and www directory
clean-all: clean-hfs clean-www