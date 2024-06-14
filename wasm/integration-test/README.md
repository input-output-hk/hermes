# WASM Integration Test

This directory comprises of integration test modules implemented for Hermes including both APIs of Hermes and WASI.

## Flow

Each module has its own `Earthfile`.
The file has the `+build` target,
to compile each module into `.wasm` component and execute with Hermes test engine.
The output of each module resides at `wasm/test-components` from the project root.
Some modules might be implemented in C or Rust or any languages that support WASM target compilation.

## Adding a new module

You can create a new directory inside `wasm/integration-test` if the test module hasn't been created yet,
or modify the existing module.
The new test module you created must have an `Earthfile` inside the directory with the `+build` target.
The `+build` target must output a WASM component you want to test with.
Make sure to setup the language of your choice properly.
When you are working with the test module, firstly, you need to generate Hermes bindings.
You can visit `wasm/wasi/Earthfile` for supported languages needed to generate Hermes bindings.

## Notes

### SQLite benchmark result

* Test: simple sequential insertion between persistent and in-memory database (100 iterations)
  * Persistent: 8,493,667 ns/iter (+/- 0)
  * In-memory: 37,492,916 ns/iter (+/- 0)

Tested on MacBook Pro with M3 Pro chip 12-core CPU, 18-core GPU, and 18GB unified memory.
