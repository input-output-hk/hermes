//! Intentionally empty
//! This file exists, so that doc tests can be used inside binary crates.
#![type_length_limit = "45079293105"]

pub mod app;
#[allow(dead_code)]
pub mod cli;
pub mod errors;
pub mod event;
pub mod hdf5;
pub mod logger;
pub mod packaging;
pub mod reactor;
pub mod runtime_context;
pub mod runtime_extensions;
pub mod utils;
pub mod vfs;
pub mod wasm;

#[cfg(feature = "bench")]
pub use wasm::module::bench::{
    module_hermes_component_bench, module_small_component_bench,
    module_small_component_full_pre_load_bench,
};
