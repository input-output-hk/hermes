# Dockerfile for Hermes P2P Testing
#
# Base image: Ubuntu 24.04 (GLIBC 2.39)
# This allows using locally-built binaries from Fedora 40+ hosts (GLIBC 2.38+)
#
# Build artifacts with:
#   just get-local-hermes-fast   # Local build (fast, compatible with Ubuntu 24.04)
#   just get-local-athena-fast   # Local WASM build + packaging
#
# Or use Earthly for containerized builds (slower but more reproducible):
#   just get-local-hermes        # Earthly containerized build
#   just get-local-athena        # Earthly WASM build

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
