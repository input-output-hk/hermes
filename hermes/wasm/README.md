# Hermes - WASM

<!-- cspell: words wasmtime wasi -->

This directory contains standalone Rust code that is not used as a project dependency.
The primary purpose of these Rust files and packages is to compile into WebAssembly (Wasm).
This code contains the forked code from
[wasmtime](https://github.com/bytecodealliance/wasmtime/tree/main/crates/wasi-preview1-component-adapter),
located in `crates/wasi-component-adapter` and `crates/wasi`.

## Configuration

The Rust configuration file locates in `.cargo/config.toml`.
It already specified the build target to `wasm32-unknown-unknown`.

To compile all the packages, simply run the command:

```bash
cargo build --release
```

To build the WebAssembly binary output `.wasm`.
