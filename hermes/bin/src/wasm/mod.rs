//! WASM related structures and functions which are specific for the Hermes use case.
//! All implementation based on [wasmtime](https://crates.io/crates/wasmtime) crate dependency.

mod engine;
pub mod module;
#[allow(dead_code, unused)]
mod patcher;
