//! WASM related structures and functions which are specific for the Hermes use case.
//! All implementation based on [wasmtime](https://crates.io/crates/wasmtime) crate dependency.

pub(crate) mod context;
mod engine;
mod module;
