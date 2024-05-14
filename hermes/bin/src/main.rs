//! The Hermes Node.

mod app;
mod cli;
mod event;
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
    use tracing::error;

    if let Err(err) = cli::Cli::parse().exec() {
        error!("{err}");
        std::process::exit(1);
    }
}
