use std::process::Command;

use temp_dir::TempDir;

use crate::utils;

#[test]
fn simple_request() {
    let tmp_dir = TempDir::new().unwrap();
    println!("Temporary directory created: {}", tmp_dir.path().display());
    const COMPONENT: &str = "http_request_rte_01";

    utils::component::build(COMPONENT, &tmp_dir).expect("failed to build component");
    utils::packaging::package_module(&tmp_dir).expect("failed to package module");
    utils::packaging::package_app().expect("failed to package app");

    let server = utils::http_server::start();

    std::thread::sleep(std::time::Duration::from_secs(60));

    // utils::hermes::run_app();
}
