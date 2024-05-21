//! The Hermes Node.

mod app;
mod cli;
mod event;
mod logger;
mod reactor;
mod runtime_context;
mod runtime_extensions;
mod wasm;

#[cfg(feature = "bench")]
pub use wasm::module::bench::{
    module_hermes_component_bench, module_small_component_bench,
    module_small_component_full_pre_load_bench,
};

/// A parameter identifier specifying the log level.
const ENV_LOG_LEVEL: &str = "HERMES_LOG_LEVEL";

#[allow(clippy::exit)]
fn main() {
    use clap::Parser;
    use tracing::error;

    use crate::logger::{self, LoggerConfigBuilder};

    let log_level = std::env::var(ENV_LOG_LEVEL)
        .unwrap_or_default()
        .parse()
        .unwrap_or_default();

    let log_config = LoggerConfigBuilder::default()
        .log_level(log_level)
        .with_thread(true)
        .with_file(true)
        .with_line_num(true)
        .build();

    if let Err(err) = logger::init(&log_config) {
        error!("{err}");
        std::process::exit(1);
    }

    if let Err(err) = cli::Cli::parse().exec() {
        error!("{err}");
        std::process::exit(1);
    }
}
