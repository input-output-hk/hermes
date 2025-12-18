# ==============================================================================
# Hermes P2P Testing - Dockerfile
# ==============================================================================
#
# CROSS-PLATFORM SUPPORT:
#   This Dockerfile copies pre-built Linux binaries from ../hermes/target/release/.
#   The justfile detects your OS and builds appropriately:
#
#   Linux:   Uses cargo directly (fast, native)
#   Mac/Win: Uses Earthly to build inside Linux containers (produces compatible binaries)
#
# USAGE:
#   just quickstart              # Auto-detects platform, builds, starts, tests
#   just build-all               # Auto-detects platform and builds
#   just build-images            # Creates Docker images from built binaries
#
# RUNTIME COMPATIBILITY:
#   - Base image: Ubuntu 24.04 (provides GLIBC 2.39)
#   - Supports binaries built on Fedora 40+ or any distro with GLIBC 2.38+
#   - GLIBC versions must match: build host ≤ runtime container for C library calls
#   - Earthly builds use consistent Linux environment → guaranteed compatibility
#
# ==============================================================================

FROM ubuntu:24.04

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Create hermes user
RUN useradd -m hermes

# Copy pre-built artifacts
# Can use either local builds (fast) or Earthly builds (reproducible)
COPY hermes/target/release/hermes /usr/local/bin/hermes
COPY hermes/apps/athena/athena.happ /app/athena.happ

# Create data directory
RUN mkdir -p /data && chown -R hermes:hermes /data /app

USER hermes
WORKDIR /data

# Default HTTP port (overridden per node via env var)
EXPOSE 5000

# IPFS ports
EXPOSE 4001 5001

# Environment variables
ENV HERMES_HTTP_PORT=5000 \
    HERMES_ACTIVATE_AUTH=true \
    REDIRECT_ALLOWED_HOSTS=app.dev.projectcatalyst.io \
    REDIRECT_ALLOWED_PATH_PREFIXES=/api/gateway/v1/config/frontend,/api/gateway/v1/cardano/assets,/api/gateway/v1/rbac/registration

ENTRYPOINT ["/usr/local/bin/hermes"]
CMD ["run", "--untrusted", "/app/athena.happ"]
