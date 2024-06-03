//! The Hermes Node.

mod app;
#[allow(dead_code)]
mod cli;
mod errors;
mod event;
mod logger;
#[allow(dead_code)]
mod packaging;
mod reactor;
mod runtime_context;
mod runtime_extensions;
mod wasm;
use crate::logger::LoggerConfigBuilder;

use std::process;

use app::HermesAppName;
use console::Emoji;
use errors::Errors;
#[cfg(feature = "bench")]
pub use wasm::module::bench::{
    module_hermes_component_bench, module_small_component_bench,
    module_small_component_full_pre_load_bench,
};

/// A parameter identifier specifying the log level.
const ENV_LOG_LEVEL: &str = "HERMES_LOG_LEVEL";

#[allow(clippy::exit)]
fn main() {
    let app_name = HermesAppName("hello world".to_string());

    println!("{}", Emoji("ℹ️", ""));

    let mut errors = Errors::new();

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

    logger::init(&log_config).unwrap_or_else(|err| errors.add_err(err));
    let module_bytes =
        include_bytes!("/home/soze/hermes-cardano-rte-test-module/my-component.wasm");
    let hermes_app =
        crate::app::HermesApp::new(app_name.clone(), vec![module_bytes.to_vec()]).expect("app");

    // Create a new reactor instance.
    let reactor_result = reactor::HermesReactor::new(vec![hermes_app]);
    let mut reactor = match reactor_result {
        Ok(reactor) => reactor,
        Err(err) => {
            println!("Error creating reactor: {}", err);
            process::exit(1);
        },
    };
    // Comment out, since it causes CI to run forever.
    if let Err(err) = reactor.wait() {
        println!("Error in reactor: {}", err);
        process::exit(1);
    }
}
