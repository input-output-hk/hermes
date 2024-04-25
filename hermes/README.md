<!-- cspell: words indexmap -->

# Hermes core

An implementation of the Hermes core engine in Rust

* [Hermes core](#hermes-core)
  * [Build notes](#build-notes)

## Build notes

During the build process, you may encounter specific known issues:
[tower/issues/466](https://github.com/tower-rs/tower/issues/466)
and [indexmap/issues/151](https://github.com/indexmap-rs/indexmap/issues/151).
These issues can impede the build's success.
We recommend explicitly setting the environment variable `CARGO_FEATURE_STD=1` as a temporary solution.
This workaround has effectively bypassed the mentioned problems until a permanent fix is implemented.

```shell
CARGO_FEATURE_STD=1 cargo b
```

## Run hermes

Hermes has a different options how to run, to inspect all of them, run the following:

```shell
hermes --help
```


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
