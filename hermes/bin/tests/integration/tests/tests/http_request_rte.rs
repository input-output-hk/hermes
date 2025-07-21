use std::process::Command;

use crate::utils;

#[test]
fn simple_request() {
    const COMPONENT: &str = "http_request_rte_01";
    utils::component::build(COMPONENT).expect("failed to build component");

    let server = utils::http_server::start();
    let hermes_binary_path = env!("CARGO_BIN_EXE_hermes");

    let output = Command::new(hermes_binary_path)
        .arg("-help")
        .output()
        .unwrap();
}
