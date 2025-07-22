use std::process::Command;

use temp_dir::TempDir;

use crate::utils::{self, assert::app_logs_contain};

#[test]
fn simple_request() {
    let tmp_dir = TempDir::new().unwrap();
    println!("Temporary directory created: {}", tmp_dir.path().display());
    const COMPONENT: &str = "http_request_rte_01";

    utils::component::build(COMPONENT, &tmp_dir).expect("failed to build component");
    utils::packaging::package_module(&tmp_dir).expect("failed to package module");
    let app_file_name = utils::packaging::package_app(&tmp_dir).expect("failed to package app");

    println!("App file created: {}", app_file_name);

    let server = utils::http_server::start();

    utils::hermes::build();
    // TODO[RC]: How do we pass server data to the app?
    // 1. VFS?
    // 2. Package into the app via metadata?
    utils::hermes::run_app(&tmp_dir, &app_file_name).expect("failed to run hermes app");

    assert!(app_logs_contain(
        &tmp_dir,
        "XXXXX - Sending HTTP request"
    ));

    println!("Now sleeping, allowing to capture the content of the temp dir before it is deleted");
    std::thread::sleep(std::time::Duration::from_secs(60));

    // utils::hermes::run_app();
}
