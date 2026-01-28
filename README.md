# Hermes

<!-- markdownlint-disable MD029 -->

Hermes is a high-performance WebAssembly (WASM) application engine that provides secure,
sandboxed execution of modular applications.

* [Hermes](#hermes)
    * [Prerequisites](#prerequisites)
        * [GitHub Token Setup](#github-token-setup)
        * [Rust Toolchain Setup](#rust-toolchain-setup)
        * [WASI SDK Setup (Local Builds Only)](#wasi-sdk-setup-local-builds-only)
    * [Quick Start](#quick-start)
    * [Build System](#build-system)
    * [Key Commands](#key-commands)
    * [Contributing](#contributing)
    * [License](#license)

## Prerequisites

Before building Hermes, ensure you have the following installed:

| Tool         | Purpose                        | Installation                                                |
| ------------ | ------------------------------ | ----------------------------------------------------------- |
| **Rust**     | Build toolchain                | [rustup.rs](https://rustup.rs)                              |
| **Just**     | Command runner                 | `cargo install just` or `brew install just`                 |
| **Docker**   | Container runtime              | [docker.com](https://www.docker.com/get-started)            |
| **Earthly**  | Containerized builds           | [earthly.dev](https://earthly.dev/get-earthly)              |
| **LLVM**     | WASM target support            | `brew install llvm` (macOS) or `apt install llvm` (Linux)   |
| **cmake**    | Native deps (local builds)     | `brew install cmake` (macOS) or `apt install cmake` (Linux) |
| **WASI SDK** | WASM C compiler (local builds) | See [WASI SDK Setup](#wasi-sdk-setup-local-builds-only)     |

### GitHub Token Setup

A GitHub token is required for accessing private dependencies:

1. Go to [github.com/settings/tokens](https://github.com/settings/tokens)
2. Generate a new **classic token** with these permissions:
   * `public_repo` - Access public repositories
   * `read:packages` - Read packages from GitHub Package Registry
3. Create a `.secret` file in the project root (use `.secret.template` as reference):

   ```bash
   cp .secret.template .secret
   # Edit .secret and add your token
   ```

### Rust Toolchain Setup

The project uses Rust 1.89 (specified in `hermes/rust-toolchain.toml`).
For local WASM builds, install the required target:

```bash
rustup install 1.89
rustup target add wasm32-wasip2 --toolchain 1.89
```

### WASI SDK Setup (Local Builds Only)

For local builds (`just build-run-dev-fastest`), the [WASI SDK](https://github.com/WebAssembly/wasi-sdk)
is required to compile C dependencies to WebAssembly.
Skip this if using containerized builds.

**Download and install** (version 29+):

<!-- markdownlint-disable MD013 -->
```bash
WASI_VERSION=29
WASI_VERSION_FULL=${WASI_VERSION}.0

# macOS (Apple Silicon)
curl -LO https://github.com/WebAssembly/wasi-sdk/releases/download/wasi-sdk-${WASI_VERSION}/wasi-sdk-${WASI_VERSION_FULL}-arm64-macos.tar.gz
tar xzf wasi-sdk-${WASI_VERSION_FULL}-arm64-macos.tar.gz
sudo mv wasi-sdk-${WASI_VERSION_FULL}-arm64-macos /opt/wasi-sdk

# macOS (Intel)
# Use: wasi-sdk-${WASI_VERSION_FULL}-x86_64-macos.tar.gz

# Linux (x86_64)
# Use: wasi-sdk-${WASI_VERSION_FULL}-x86_64-linux.tar.gz

# Linux (arm64)
# Use: wasi-sdk-${WASI_VERSION_FULL}-arm64-linux.tar.gz
```

See [WASI SDK Releases](https://github.com/WebAssembly/wasi-sdk/releases) for all available platforms.

> **Note:** make sure wasi-sdk is installed to `/opt/wasi-sdk`.

**Set environment variables** (add to `~/.zshrc`, `~/.bashrc`, or `~/.profile`):

```bash
export WASI_SDK_PATH="/opt/wasi-sdk"
export CC_wasm32_wasip2="${WASI_SDK_PATH}/bin/clang"
```

Then reload your shell: `source ~/.zshrc` (or restart your terminal)

## Quick Start

```bash
just check-local-build       # Verify your setup (run once)
just build-run-dev           # 🐳 Recommended: containerized build (first-time setup)
just build-run-dev-fastest   # 🚀 Fast iteration (requires local WASM toolchain)
```

> **Tip:** Use `just --list` to see all available commands, `just --show <command>` for details.

## Build System

This project uses [Just](https://github.com/casey/just) with two build approaches:

| Approach            | When to Use                            | Requirements                 |
| ------------------- | -------------------------------------- | ---------------------------- |
| 🐳 **Containerized** | First-time setup, CI/CD, final testing | Docker/Podman + Earthly      |
| 🚀 **Local**         | Daily development, rapid iteration     | Local Rust + `wasm32-wasip2` |

## Key Commands

| Command                      | Description                          |
| ---------------------------- | ------------------------------------ |
| `just build-run-dev`         | 🐳 Containerized build (recommended)  |
| `just build-run-dev-fastest` | 🚀 Local build (fast iteration)       |
| `just build-run-all`         | 📦 Production build (full assets)     |
| `just dev-athena-fast`       | WASM modules only (dev)              |
| `just dev-athena`            | WASM modules only (prod)             |
| `just check-local-build`     | Verify local Rust and WASM toolchain |
| `just status`                | Show build status                    |
| `just clean-hfs`             | Clean application state              |

## Contributing

We welcome contributions from the community!
Please read our [CONTRIBUTING](CONTRIBUTING.md) for guidelines on how to contribute.

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or
[MIT license](LICENSE-MIT) at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in this crate by you, as defined in the Apache-2.0 license, shall
be dual licensed as above, without any additional terms or conditions.
