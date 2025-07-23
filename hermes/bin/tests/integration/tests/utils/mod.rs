use temp_dir::TempDir;

pub mod assert;
pub mod component;
pub mod hermes;
pub mod http_server;
pub mod packaging;

pub const HERMES_BINARY_PATH: &str = env!("CARGO_BIN_EXE_hermes");
pub const LOG_FILE_NAME: &str = "app_output.log";

pub fn debug_sleep(temp_dir: &TempDir) {
    println!(
        "Now sleeping for 60 sec., allowing to capture the content of the temp dir ({}) before it is deleted",
        temp_dir.as_ref().display()
    );
    std::thread::sleep(std::time::Duration::from_secs(60));
}
