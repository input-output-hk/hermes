//! The Hermes Node.

mod app;
mod event;
mod logger;
mod reactor;
mod runtime_context;
mod runtime_extensions;
mod wasm;

use tracing::{span, Level};
#[cfg(feature = "bench")]
pub use wasm::module::bench::{
    module_hermes_component_bench, module_small_component_bench,
    module_small_component_full_pre_load_bench,
};

build_info::build_info!(fn build_info);

fn main() {
    let mut reactor = reactor::HermesReactor::new(Vec::new()).unwrap();
    println!("Hello, world!");
    logger::init(logger::LogFormat::Json, logger::LogLevel::Info).unwrap();
    span!(Level::INFO, "my_span");
    println!("{:#?}", build_info());

    reactor.wait().unwrap();
}
