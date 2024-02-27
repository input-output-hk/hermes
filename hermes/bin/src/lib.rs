//! Intentionally empty
//! This file exists, so that doc tests can be used inside binary crates.

mod event_queue;
mod runtime_extensions;
mod state;
mod wasm;

#[cfg(feature = "bench")]
pub use wasm::module::bench::{
    module_hermes_component_bench, module_small_component_bench,
    module_small_component_full_pre_load_bench,
};
