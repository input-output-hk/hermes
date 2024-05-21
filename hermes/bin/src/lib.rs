//! Intentionally empty
//! This file exists, so that doc tests can be used inside binary crates.

pub mod app;
#[allow(dead_code)]
pub mod cli;
pub mod event;
#[allow(dead_code)]
pub mod logger;
pub mod reactor;
pub mod runtime_context;
pub mod runtime_extensions;
pub mod wasm;

#[cfg(feature = "bench")]
pub use wasm::module::bench::{
    module_hermes_component_bench, module_small_component_bench,
    module_small_component_full_pre_load_bench,
};
