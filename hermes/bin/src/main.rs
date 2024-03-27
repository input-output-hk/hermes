//! The Hermes Node.

mod app;
mod event;
mod logger;
mod reactor;
mod runtime_context;
mod runtime_extensions;
mod wasm;

use std::{env, process, str::FromStr};

use tracing::{error, info};
#[cfg(feature = "bench")]
pub use wasm::module::bench::{
    module_hermes_component_bench, module_small_component_bench,
    module_small_component_full_pre_load_bench,
};

use crate::logger::{LogLevel, LoggerConfig};

// use crate::logger::{LogLevel, LoggerConfigBuilder};

build_info::build_info!(fn build_info);

/// A parameter identifier specifying the log level.
const ENV_LOG_LEVEL: &str = "LOG_LEVEL";
/// The default value for the log level when not specified.
const DEFAULT_ENV_LOG_LEVEL: &str = "info";

// Disable process exit for clippy.
#[allow(clippy::exit)]
fn main() {
    let log_level = env::var(ENV_LOG_LEVEL).unwrap_or_else(|_| DEFAULT_ENV_LOG_LEVEL.to_owned());

    let config = LoggerConfig::default()
        .log_level(LogLevel::from_str(&log_level).unwrap_or_default())
        .with_thread(true)
        .with_file(true)
        .with_line_num(true)
        .build();

    // Initialize logger.
    if let Err(err) = logger::init(&config) {
        println!("Error initializing logger: {err}");
    }
    // Get build info string.
    let build_info_str = format!("{:?}", build_info());

    // Create a new reactor instance.
    let reactor_result = reactor::HermesReactor::new(Vec::new());
    let mut _reactor = match reactor_result {
        Ok(reactor) => reactor,
        Err(err) => {
            error!("Error creating reactor: {}", err);
            process::exit(1);
        },
    };

    info!("{}", build_info_str);

    // if let Err(err) = reactor.wait() {
    //     error!("Error in reactor: {}", err);
    //     process::exit(1);
    // }
}
