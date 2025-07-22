pub mod assert;
pub mod component;
pub mod hermes;
pub mod http_server;
pub mod packaging;

pub const HERMES_BINARY_PATH: &str = env!("CARGO_BIN_EXE_hermes");
pub const LOG_FILE_NAME: &str = "app_output.log";