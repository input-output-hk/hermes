use std::process::Command;

use temp_dir::TempDir;

use crate::utils;

#[test]
fn simple_request() {
    let temp_dir = TempDir::new().unwrap();
    const COMPONENT: &str = "http_request_rte_01";
    println!("temp_dir: {}", temp_dir.as_ref().display());

    utils::component::build(COMPONENT, &temp_dir).expect("failed to build component");
    let app_file_name = utils::packaging::package(&temp_dir).expect("failed to package app");

    let server = utils::http_server::start();
    utils::component::set("http_server", &server.base_url(), &temp_dir).expect("set failed");

    // TODO[RC]: Build hermes once for all tests
    utils::hermes::build();

    // TODO[RC]: How do we pass server data to the app?
    // 1. VFS?
    // 2. Package into the app via metadata?
    utils::hermes::run_app(&temp_dir, &app_file_name).expect("failed to run hermes app");

    assert!(utils::assert::app_logs_contain(
        &temp_dir,
        "XXXXX - Sending HTTP request"
    ));

    utils::debug_sleep(&temp_dir);
}
