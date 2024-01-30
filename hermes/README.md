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
