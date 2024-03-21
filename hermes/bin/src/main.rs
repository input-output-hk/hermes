//! The Hermes Node.

mod app;
mod event;
mod logger;
mod reactor;
mod runtime_context;
mod runtime_extensions;
mod wasm;

use std::process;

use tracing::{error, span, Level};
#[cfg(feature = "bench")]
pub use wasm::module::bench::{
    module_hermes_component_bench, module_small_component_bench,
    module_small_component_full_pre_load_bench,
};

build_info::build_info!(fn build_info);

// Disable process exit for clippy.
#[allow(clippy::exit)]
fn main() {
    // Initialize logger.
    if let Err(err) = logger::init(logger::LogLevel::Info) {
        error!("Error initializing logger: {}", err);
    }

    // Create a new reactor instance.
    let reactor_result = reactor::HermesReactor::new(Vec::new());
    let mut reactor = match reactor_result {
        Ok(reactor) => reactor,
        Err(err) => {
            error!("Error creating reactor: {}", err);
            process::exit(1);
        },
    };

    // Get build info string.
    let build_info_str = format!("{:?}", build_info());

    // Start application span
    span!(Level::INFO, "Start Application", build_info_str);

    if let Err(err) = reactor.wait() {
        error!("Error in reactor: {}", err);
        process::exit(1);
    }
}
