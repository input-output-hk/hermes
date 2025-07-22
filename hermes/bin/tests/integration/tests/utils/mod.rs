pub mod component;
pub mod http_server;
pub mod packaging;
pub mod hermes;

pub const HERMES_BINARY_PATH: &str = env!("CARGO_BIN_EXE_hermes");
