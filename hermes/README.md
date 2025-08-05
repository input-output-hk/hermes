<!-- cspell: words indexmap -->



## Running benchmarks

Before running benchmarks need to compile a simple WASM module:

```shell
earthly ./wasm/c+save-local
```

And then you can run benchmarks:

```shell
cargo bench --features bench
```

## Running integration tests

Before running integration tests make sure to compile test modules:

```shell
earthly ./wasm+save-c-integration-test-local
```

And then you can run integration tests:

```shell
cargo test
```

### Environment Variables

* `TEST_WASM_MODULE_DIR`: Specifies the directory for placing test WebAssembly components.
  Default value: "../../wasm/test-components".
* `N_TEST`: Specifies the number of tests to run.
  Default value: `32`.
* `N_BENCH`: Specifies the number of benchmarks to run.
  Default value: `32`.

Example usage on using an env variable to specify the specific test components:

```shell
env TEST_WASM_MODULE_DIR=tests/test-components bash -c "cargo test"
```
