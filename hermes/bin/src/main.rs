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
    logger::init(logger::LogLevel::Info).unwrap();
    let build_info_str = format!("{:?}", build_info());
    span!(Level::INFO, "Start Application", build_info_str);
    reactor.wait().unwrap();
}
