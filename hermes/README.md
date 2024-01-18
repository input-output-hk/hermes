<!-- cspell: words indexmap -->

# Hermes core

An implementation of the Hermes core engine in Rust

## Build notes

Unfortunately during the build process you could face with the known problem
[tower/issues/466](https://github.com/tower-rs/tower/issues/466),
[indexmap/issues/151](https://github.com/indexmap-rs/indexmap/issues/151).
The only workaround for now is to explicitly provide `CARGO_FEATURE_STD=1`.

```shell
CARGO_FEATURE_STD=1 cargo b
```
