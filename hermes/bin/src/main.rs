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
    use std::str::FromStr;

    use clap::Parser;
    use tracing::error;

    use crate::logger::{self, LogLevel, LoggerConfigBuilder};

    let log_level = if let Ok(log_level_str) = std::env::var(ENV_LOG_LEVEL) {
        LogLevel::from_str(&log_level_str)?
    } else {
        LogLevel::default()
    };

    let log_config = LoggerConfigBuilder::default()
        .log_level(log_level)
        .with_thread(true)
        .with_file(true)
        .with_line_num(true)
        .build();
    logger::init(&log_config)?;

    if let Err(err) = cli::Cli::parse().exec() {
        error!("{err}");
        std::process::exit(1);
    }
}
