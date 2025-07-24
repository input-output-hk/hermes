use temp_dir::TempDir;

use crate::utils;

#[test]
fn simple_request() {
    let temp_dir = TempDir::new().unwrap();
    const COMPONENT: &str = "http_request_rte_01";

    utils::component::build(COMPONENT, &temp_dir).expect("failed to build component");
    let server = utils::http_server::start();
    utils::component::set("http_server", &server.base_url(), &temp_dir).expect("set failed");
    let app_file_name = utils::packaging::package(&temp_dir).expect("failed to package app");

    // TODO[RC]: Build hermes just once for all tests
    utils::hermes::build();

    utils::hermes::run_app(&temp_dir, &app_file_name).expect("failed to run hermes app");

    assert!(utils::assert::app_logs_contain(
        &temp_dir,
        "[TEST] got response with request_id"
    ));

    assert!(utils::assert::app_logs_contain(
        &temp_dir,
        "This is the content of the 'test.txt' file"
    ));

    // Uncomment the line below if you want to inspect the details
    // available in the temp directory.
    // 
    // utils::debug_sleep(&temp_dir);
}
