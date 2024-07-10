//! The Hermes Node.

mod app;
mod cli;
mod errors;
mod event;
mod hdf5;
mod logger;
mod packaging;
mod reactor;
mod runtime_context;
mod runtime_extensions;
mod wasm;

#[cfg(feature = "bench")]
pub use wasm::module::bench::{
    module_hermes_component_bench, module_small_component_bench,
    module_small_component_full_pre_load_bench,
};

#[allow(clippy::exit)]
fn main() {
    use clap::Parser;

    cli::Cli::parse().exec();
}
