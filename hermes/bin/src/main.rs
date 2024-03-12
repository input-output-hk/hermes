//! The Hermes Node.

mod app;
mod event;
mod reactor;
mod runtime_context;
mod runtime_extensions;
mod wasm;

#[cfg(feature = "bench")]
pub use wasm::module::bench::{
    module_hermes_component_bench, module_small_component_bench,
    module_small_component_full_pre_load_bench,
};

fn main() {
    println!("Hello, world!");
}
