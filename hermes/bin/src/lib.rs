//! Intentionally empty
//! This file exists, so that doc tests can be used inside binary crates.

pub mod app;
pub mod cli;
pub mod errors;
pub mod event;
pub mod logger;
pub mod packaging;
pub mod reactor;
pub mod runtime_context;
pub mod runtime_extensions;
pub mod wasm;

#[cfg(feature = "bench")]
pub use wasm::module::bench::{
    module_hermes_component_bench, module_small_component_bench,
    module_small_component_full_pre_load_bench,
};
