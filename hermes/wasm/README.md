# Hermes - WASM

<!-- cspell: words wasmtime wasi -->

This directory contains standalone Rust code that is not used as a project dependency.
The primary purpose of these Rust files and packages is to compile into WebAssembly (Wasm).
This code contains the forked code from
[wasmtime](https://github.com/bytecodealliance/wasmtime/tree/main/crates/wasi-preview1-component-adapter),
located in `crates/wasi-component-adapter` and `crates/wasi`.

## Configuration

To compile, simply run the command:

```bash
cargo build --target=wasm32-unknown-unknown --release
```

To build the WebAssembly binary output `.wasm`.
