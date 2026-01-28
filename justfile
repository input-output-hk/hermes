# Hermes Applications Build System
#
# Two build approaches:
# 🚀 LOCAL: build-run-dev-fastest (uses local Rust, fastest)
# 🐳 CONTAINERIZED: build-run-dev (uses Earthly, matches CI)
#
# Quick start:
#   just check-local-build     # Verify local setup (run once) 
#   just build-run-dev-fastest # Fastest dev workflow
#   just build-run-dev         # Reliable dev workflow  
#   just build-run-all         # Production build

default:
    @just --list --unsorted

# Build Hermes binary using Earthly (containerized)
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

# Internal: Build all WASM modules (prod=full assets, dev=minimal assets)
_build-athena-common mode:
    #!/usr/bin/env bash
    set -euo pipefail

    # Set build-specific variables
    if [ "{{mode}}" = "prod" ]; then
        HTTP_PROXY_TARGET="local-build-http-proxy"
        BUILD_TYPE="PRODUCTION"
        ASSETS_DESC="includes all web assets"
        SUCCESS_MSG="🎉 PRODUCTION build and packaging complete!"
        ASSETS_STATUS="🌐 Includes: Full web assets (assets/, canvaskit/, icons/)"
    else
        HTTP_PROXY_TARGET="local-build-http-proxy-dev"
        BUILD_TYPE="DEV MODE"
        ASSETS_DESC="minimal web assets"
        SUCCESS_MSG="🎉 DEVELOPMENT build and packaging complete!"
        ASSETS_STATUS="⚡ Development build: Uses placeholder web assets for faster iteration"
        echo "⚡ Development mode: Skipping large web assets for faster builds"
    fi

    echo "🔨 Building HTTP proxy WASM component ($BUILD_TYPE)..."
    echo "📍 Module location: athena/modules"
    echo "🎯 Target: wasm32-wasip2 (WebAssembly System Interface Preview 2)"

    # Step 1: Build all WASM modules using Earthly IN PARALLEL
    # The HTTP proxy module uses different targets for prod vs dev (web assets)
    # Other modules use the same target regardless of mode
    echo "🔧 Building WASM modules in parallel..."
    echo "⚡ Starting 5 concurrent Earthly builds for faster compilation"
    
    # PARALLEL EXECUTION PATTERN:
    # 1. Launch each build in a sub shell with '&' to run in background
    # 2. Capture the Process ID (PID) with '$!' for each background job
    # 3. Use 'wait $PID' to synchronize and collect exit codes
    # 4. This allows all 5 modules to compile simultaneously instead of sequentially
    #    Typical speedup: 5x faster than sequential builds
    
    # Start all builds in background processes (non-blocking)
    (
        echo "  📦 Building http-proxy module..."
        earthly ./hermes/apps/athena/modules/http-proxy+$HTTP_PROXY_TARGET 
    ) &
    HTTP_PROXY_PID=$!  # Capture PID of background process
    
    (
        echo "  📦 Building rbac-registration-indexer module..."
        earthly ./hermes/apps/athena/modules/rbac-registration-indexer+local-build-rbac-registration-indexer
    ) &
    RBAC_INDEXER_PID=$!  # Capture PID of background process
    
    (
        echo "  📦 Building rbac-registration module..."
        earthly ./hermes/apps/athena/modules/rbac-registration+local-build-rbac-registration
    ) &
    RBAC_PID=$!  # Capture PID of background process
    
    (
        echo "  📦 Building staked-ada-indexer module..."
        earthly ./hermes/apps/athena/modules/staked-ada-indexer+local-build-staked-ada-indexer
    ) &
    STAKED_INDEXER_PID=$!  # Capture PID of background process
    
    (
        echo "  📦 Building staked-ada module..."
        earthly ./hermes/apps/athena/modules/staked-ada+local-build-staked-ada
    ) &
    STAKED_PID=$!  # Capture PID of background process

    (
        echo "  📦 Building auth module..."
        earthly ./hermes/apps/athena/modules/auth+local-build-auth
    ) &
    AUTH_PID=$!  # Capture PID of background process

    (
        echo "  📦 Building doc-sync module..."
        earthly ./hermes/apps/athena/modules/doc-sync+local-build-doc-sync
    ) &
    DOC_SYNC_PID=$!  # Capture PID of background process

    # SYNCHRONIZATION PHASE:
    # Wait for all background jobs to complete and collect their exit codes
    # Track failures so we can exit with an error if any build fails
    echo "⏳ Waiting for all parallel builds to complete..."

    BUILD_FAILED=0

    # Wait for each process and report completion status
    # 'wait $PID' blocks until that specific process finishes and returns its exit code
    wait $HTTP_PROXY_PID && echo "  ✅ http-proxy build completed" || { echo "  ❌ http-proxy build failed"; BUILD_FAILED=1; }
    wait $RBAC_INDEXER_PID && echo "  ✅ rbac-registration-indexer build completed" || { echo "  ❌ rbac-registration-indexer build failed"; BUILD_FAILED=1; }
    wait $RBAC_PID && echo "  ✅ rbac-registration build completed" || { echo "  ❌ rbac-registration build failed"; BUILD_FAILED=1; }
    wait $STAKED_INDEXER_PID && echo "  ✅ staked-ada-indexer build completed" || { echo "  ❌ staked-ada-indexer build failed"; BUILD_FAILED=1; }
    wait $STAKED_PID && echo "  ✅ staked-ada build completed" || { echo "  ❌ staked-ada build failed"; BUILD_FAILED=1; }
    wait $AUTH_PID && echo "  ✅ auth build completed" || { echo "  ❌ auth build failed"; BUILD_FAILED=1; }
    wait $DOC_SYNC_PID && echo "  ✅ doc-sync build completed" || { echo "  ❌ doc-sync build failed"; BUILD_FAILED=1; }

    if [ $BUILD_FAILED -eq 1 ]; then
        echo "🚨 One or more module builds failed. Aborting."
        exit 1
    fi

    echo "🎯 All parallel builds completed!"

    echo "✅ WASM compilation complete ($BUILD_TYPE - $ASSETS_DESC)"

    echo "📦 Packaging modules with Hermes CLI in parallel..."
    echo "📄 Using manifests from hermes/apps/athena/modules/*/lib/manifest_module.json"

    # Step 2: Package all WASM modules with their configurations into .hmod format IN PARALLEL
    # The .hmod files contain the WASM binary, manifest, and metadata for each module
    # This step takes the compiled WASM files and bundles them with their configuration
    echo "⚡ Starting 7 concurrent module packaging operations..."

    # PARALLEL PACKAGING PATTERN:
    # Same approach as the build step above, but for the Hermes CLI packaging operations
    # Each 'hermes module package' command:
    # 1. Reads the manifest_module.json configuration file
    # 2. Locates the corresponding .wasm file (compiled in previous step)
    # 3. Creates a .hmod file containing both the WASM binary and metadata
    # 4. Validates the package structure and dependencies
    # Running these in parallel saves significant time when packaging multiple modules

    # Start all packaging operations in background processes (non-blocking)
    (
        echo "  📦 Packaging http-proxy module..."
        target/release/hermes module package hermes/apps/athena/modules/http-proxy/lib/manifest_module.json
    ) &
    HTTP_PROXY_PKG_PID=$!  # Capture PID for synchronization

    (
        echo "  📦 Packaging rbac-registration-indexer module..."
        target/release/hermes module package hermes/apps/athena/modules/rbac-registration-indexer/lib/manifest_module.json
    ) &
    RBAC_INDEXER_PKG_PID=$!  # Capture PID for synchronization

    (
        echo "  📦 Packaging rbac-registration module..."
        target/release/hermes module package hermes/apps/athena/modules/rbac-registration/lib/manifest_module.json
    ) &
    RBAC_PKG_PID=$!  # Capture PID for synchronization

    (
        echo "  📦 Packaging staked-ada-indexer module..."
        target/release/hermes module package hermes/apps/athena/modules/staked-ada-indexer/lib/manifest_module.json
    ) &
    STAKED_INDEXER_PKG_PID=$!  # Capture PID for synchronization

    (
        echo "  📦 Packaging staked-ada module..."
        target/release/hermes module package hermes/apps/athena/modules/staked-ada/lib/manifest_module.json
    ) &
    STAKED_PKG_PID=$!  # Capture PID for synchronization

    (
        echo "  📦 Packaging auth module..."
        target/release/hermes module package hermes/apps/athena/modules/auth/lib/manifest_module.json
    ) &
    AUTH_PKG_PID=$!  # Capture PID for synchronization

    (
        echo "  📦 Packaging doc-sync module..."
        target/release/hermes module package hermes/apps/athena/modules/doc-sync/lib/manifest_module.json
    ) &
    DOC_SYNC_PKG_PID=$!  # Capture PID for synchronization

    # SYNCHRONIZATION PHASE:
    # Wait for all packaging processes to complete before proceeding to app packaging
    # This ensures all .hmod files are ready before the final .happ creation
    echo "⏳ Waiting for all parallel packaging operations to complete..."

    PKG_FAILED=0

    # Wait for each packaging process and report completion status
    # Each 'wait' command blocks until that specific packaging operation finishes
    wait $HTTP_PROXY_PKG_PID && echo "  ✅ http-proxy packaging completed" || { echo "  ❌ http-proxy packaging failed"; PKG_FAILED=1; }
    wait $RBAC_INDEXER_PKG_PID && echo "  ✅ rbac-registration-indexer packaging completed" || { echo "  ❌ rbac-registration-indexer packaging failed"; PKG_FAILED=1; }
    wait $RBAC_PKG_PID && echo "  ✅ rbac-registration packaging completed" || { echo "  ❌ rbac-registration packaging failed"; PKG_FAILED=1; }
    wait $STAKED_INDEXER_PKG_PID && echo "  ✅ staked-ada-indexer packaging completed" || { echo "  ❌ staked-ada-indexer packaging failed"; PKG_FAILED=1; }
    wait $STAKED_PKG_PID && echo "  ✅ staked-ada packaging completed" || { echo "  ❌ staked-ada packaging failed"; PKG_FAILED=1; }
    wait $AUTH_PKG_PID && echo "  ✅ auth packaging completed" || { echo "  ❌ auth packaging failed"; PKG_FAILED=1; }
    wait $DOC_SYNC_PKG_PID && echo "  ✅ doc-sync packaging completed" || { echo "  ❌ doc-sync packaging failed"; PKG_FAILED=1; }

    if [ $PKG_FAILED -eq 1 ]; then
        echo "🚨 One or more module packaging operations failed. Aborting."
        exit 1
    fi

    echo "🎯 All parallel packaging operations completed!"
    echo "✅ Module packaging complete (.hmod files created)"

    echo "📦 Packaging application bundle..."
    echo "📄 Using manifest: hermes/apps/athena/manifest_app.json"

    # Step 3: Create final application package (.happ file)
    # The .happ file bundles all modules and application-level configuration
    target/release/hermes app package hermes/apps/athena/manifest_app.json
    echo "✅ Application packaging complete (.happ file created)"

    echo "$SUCCESS_MSG"
    echo "📦 Application package: hermes/apps/athena/athena.happ"
    echo "📏 Package size: $(ls -lh hermes/apps/athena/athena.happ | awk '{print $5}' 2>/dev/null || echo 'N/A')"
    echo "$ASSETS_STATUS"

# Build WASM modules with full web assets (production)
get-local-athena: (_build-athena-common "prod")

# Build WASM modules with minimal web assets (development)
get-local-athena-dev: (_build-athena-common "dev")

# Clean Hermes state files from ~/.hermes/
clean-hfs:
    @echo "🧹 Cleaning Hermes state files..."
    @if [ -d ~/.hermes ]; then \
        echo "📁 Found ~/.hermes/ directory"; \
        find ~/.hermes -name "*.hfs" -type f -delete 2>/dev/null || true; \
        echo "✅ Cleaned up .hfs files from ~/.hermes/"; \
    else \
        echo "ℹ️  ~/.hermes/ directory does not exist (nothing to clean)"; \
    fi

# Run the Athena application
# Test with: curl -H "Host: app.hermes.local" http://localhost:7878/api/gateway/v1/rbac/registration
run-athena:
    #!/usr/bin/env bash
    set -euo pipefail

    echo "🚀 Running Athena application..."
    echo "📦 Package: hermes/apps/athena/athena.happ"
    echo "🔒 Security: Running with --untrusted flag (maximum isolation)"

    # Validate prerequisites
    if [ ! -f "target/release/hermes" ]; then
        echo "❌ Error: Hermes binary not found. Run 'just get-local-hermes' first."
        exit 1
    fi

    if [ ! -f "hermes/apps/athena/athena.happ" ]; then
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
    echo "💡 Test with: curl -H 'Host: app.hermes.local' http://localhost:7878/api/gateway/v1/rbac/registration"
    echo "🛑 Press Ctrl+C to stop"
    echo ""

    # Execute the application with security sandboxing
    # HERMES_LOG_LEVEL="debug"
    target/release/hermes run --untrusted hermes/apps/athena/athena.happ



# Production build: full web assets, containerized (slow but complete)
# Use for: final testing, deployments, CI/CD pipelines
build-run-all: clean-hfs clean-wasm get-local-hermes get-local-athena clean-www run-athena

# Development build: minimal assets, containerized (reliable, matches CI)
# Use for: team consistency, when local builds fail, final testing before PRs
build-run-dev: clean-hfs clean-wasm get-local-hermes get-local-athena-dev clean-www run-athena

# Quick WASM rebuild with full assets (skips engine rebuild)
#
# Workflow: Build WASM → Package → Run
# Duration: ~5-10 minutes (skips Hermes engine compilation, includes web assets)
# Quick WASM rebuild with full assets
dev-athena: get-local-athena run-athena

# Quick WASM rebuild with minimal assets (skips engine rebuild)
#
# Workflow: Build WASM → Package → Run  
# Duration: ~2-4 minutes (skips Hermes engine compilation and web assets)
# Quick WASM rebuild with minimal assets (development)
dev-athena-fast: get-local-athena-dev run-athena

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
    if [ -f "athena/athena.happ" ]; then
        echo "   ✅ Package: $(ls -lh athena/athena.happ | awk '{print $5 " " $6 " " $7 " " $8}')"
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
    echo "   Allowed Paths: ${REDIRECT_ALLOWED_PATH_PREFIXES:-/api/gateway/v1/... (default)}"
    echo ""

    echo "💡 Quick Commands:"
    echo "   just build-run-dev-fastest  # Fastest dev build (local builds)"
    echo "   just build-run-dev          # Reliable dev build (containerized, matches CI)"
    echo "   just build-run-all          # Production build (slow, full assets)"
    echo "   just dev-athena-fast        # Quick WASM rebuild (dev mode)"
    echo "   just dev-athena             # Quick WASM rebuild (prod mode)"
    echo "   just clean-hfs              # Clear application state"

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

# Clean stale WASM modules to prevent WASI version mismatch errors
#
# Compares cached .wasm files against shared/Cargo.toml (which specifies the
# wit-bindgen version). Only cleans modules older than this marker file.
#
# This runs automatically in build-run-all, build-run-dev, build-run-dev-fastest.
# Preserves cache when modules are up-to-date, rebuilds only when wit-bindgen
# version changes.
clean-wasm:
    #!/usr/bin/env bash
    set -euo pipefail
    # Only clean WASM modules older than shared/Cargo.toml (wit-bindgen version marker)
    STALE=$(find hermes/apps/athena/modules -path "*/lib/*.wasm" -type f ! -newer hermes/apps/athena/shared/Cargo.toml 2>/dev/null)
    if [ -n "$STALE" ]; then
        echo "🧹 Cleaning stale WASM modules..."
        echo "$STALE" | xargs rm -f
    fi

# Enhanced cleanup that includes HFS files, www directory, and stale WASM modules
clean-all: clean-hfs clean-www clean-wasm

# LOCAL DEV BUILDS - Use local Rust instead of containers
# Prerequisites: run `just check-local-build` first

# Build Hermes locally
get-local-hermes-fast:
    #!/usr/bin/env bash
    set -euo pipefail
    
    echo "🚀 Building Hermes locally (bypassing Earthly container)..."
    
    cd hermes
    
    # Build with local Rust toolchain (much faster than container)
    cargo build --release --bin hermes
    
    # Copy to expected location  
    mkdir -p ../target/release
    cp target/release/hermes ../target/release/hermes
    chmod +x ../target/release/hermes
    
    echo "✅ Local Hermes build complete!"
    echo "⚡ Built with local Rust toolchain"

# Build WASM modules locally  
get-local-athena-fast:
    #!/usr/bin/env bash
    set -euo pipefail
    
    echo "🚀 Building WASM modules locally (bypassing Earthly containers)..."
    
    # Ensure WASM target is installed
    rustup target add wasm32-wasip2 || true
    
    cd hermes/apps/athena
    
    echo "🔧 Building all modules locally (sequential to avoid lock contention)..."
    
    # Build all modules in one go to share compilation cache
    echo "  📦 Building all WASM modules with shared cache..."
    cargo build --target wasm32-wasip2 --release
    
    # Now copy the WASM files to each module's lib directory
    echo "  📎 Setting up module lib directories..."
    
    # HTTP Proxy
    (
        cd modules/http-proxy
        mkdir -p lib/www/{assets,canvaskit,icons}
        
        # Copy WASM file
        cp "../../target/wasm32-wasip2/release/http_proxy.wasm" "lib/http_proxy.wasm"
        
        # Create dev web assets (minimal placeholders)
        echo '{"dev": "placeholder"}' > lib/www/assets/placeholder.json
        echo '{"dev": "placeholder"}' > lib/www/canvaskit/placeholder.json
        echo '{"dev": "placeholder"}' > lib/www/icons/placeholder.json
        
        # Copy config files (avoid copying to same location)
        echo '{}' > lib/config.json
        
        echo "  ✅ http-proxy setup complete"
    )
    
    # Other modules
    for module in doc-sync rbac-registration-indexer rbac-registration staked-ada-indexer staked-ada auth; do
        (
            cd modules/$module
            mkdir -p lib
            
            # Convert module name to WASM file name (replace hyphens with underscores)
            wasm_name=$(echo $module | tr '-' '_')
            
            # Copy WASM file
            if [ -f "../../target/wasm32-wasip2/release/${wasm_name}.wasm" ]; then
                cp "../../target/wasm32-wasip2/release/${wasm_name}.wasm" "lib/${wasm_name}.wasm"
                echo "  ✅ $module setup complete (${wasm_name}.wasm)"
            else
                echo "  ⚠️  $module: WASM file not found (${wasm_name}.wasm)"
                echo "       Available files:" && ls -1 "../../target/wasm32-wasip2/release/"*.wasm 2>/dev/null | head -5 || echo "No .wasm files"
            fi
        )
    done
    
    echo "✅ All WASM modules built locally!"
    echo "⚡ Built with local Rust toolchain"

# Fastest dev build using local Rust (recommended for daily dev)
# Use for: daily development, rapid iteration, maximum productivity
build-run-dev-fastest: clean-hfs clean-wasm get-local-hermes-fast get-local-athena-fast clean-www
    #!/usr/bin/env bash
    set -euo pipefail
    
    echo "📦 Packaging modules..."
    
    cd hermes/apps/athena
    
    # Package modules in parallel
    for module_path in modules/*/lib/manifest_module.json; do
        if [ -f "$module_path" ]; then
            echo "  📦 Packaging $(dirname $(dirname $module_path))..."
            ../../../target/release/hermes module package "$module_path" &
        fi
    done
    wait
    
    # Package application
    echo "📦 Packaging application..."
    ../../../target/release/hermes app package manifest_app.json
    
    echo "✅ Fastest build complete! 🚀"
    echo "📊 Built with local Rust toolchain"
    
    # Run
    cd ../../..
    just run-athena

# Verify local Rust setup (run once before using *-fastest commands)
check-local-build:
    #!/usr/bin/env bash
    set -euo pipefail
    
    echo "🔍 Checking local build requirements..."
    
    # Check Rust
    if command -v rustc >/dev/null 2>&1; then
        echo "✅ Rust: $(rustc --version)"
    else
        echo "❌ Rust not found. Install from: https://rustup.rs/"
        exit 1
    fi
    
    # Check WASM target
    if rustup target list --installed | grep -q wasm32-wasip2; then
        echo "✅ wasm32-wasip2 target installed"
    else
        echo "🔧 Installing wasm32-wasip2 target..."
        rustup target add wasm32-wasip2
    fi
    
    # Test build
    echo "🧪 Testing quick build..."
    cd hermes
    if timeout 30 cargo check --target wasm32-wasip2 >/dev/null 2>&1; then
        echo "✅ Local WASM build is possible!"
        echo ""
        echo "💡 Use these super-fast commands:"
        echo "   just build-run-dev-fastest     # Fastest possible (local builds)"
        echo "   just get-local-hermes-fast     # Just rebuild Hermes locally"
        echo "   just get-local-athena-fast     # Just rebuild WASM locally"
        echo ""
        echo "⚡ Local builds ready - much faster than containerized builds!"
    else
        echo "⚠️  Local WASM build may need WASI setup. Try the hybrid approach."
        echo "   You can still use: just build-run-dev (uses Earthly for WASM)"
    fi
