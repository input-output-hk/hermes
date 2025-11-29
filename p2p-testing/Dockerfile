# Dockerfile for Hermes P2P Testing
#
# CRITICAL: This Dockerfile expects pre-built artifacts from Earthly (containerized builds).
# Do NOT use locally-built binaries (cargo build) as they may have GLIBC incompatibilities.
#
# Build artifacts with:
#   just get-local-hermes    # Earthly containerized build (GLIBC-compatible)
#   just get-local-athena    # Earthly WASM build + packaging
#
# Why Earthly? Ensures binaries built in a controlled environment match the GLIBC
# version in this Debian Bookworm container, regardless of host OS (Fedora, Ubuntu, etc.)

FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Create hermes user
RUN useradd -m -u 1000 hermes

# Copy Earthly-built artifacts (GLIBC-compatible with Debian Bookworm)
# IMPORTANT: These MUST be built with Earthly, not local cargo builds
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
