use serial_test::serial;
use temp_dir::TempDir;

use crate::utils;

// TODO[RC]: Other cases to test (and fix, if issues are found):
// Alternatively, switch to using some battle-testes library like `reqwest` or `curl`
// - GET/POST
// - http/https
// - misbehaving server
//   - no response / server hang indefinitely
//   - broken communication
//   - malformed response, not valid UTF-8 or not according to the HTTP spec, mismatched
//     content-length
//   - connection timeout, slowloris attack
//   - super large response (will cause memory issues since we just do `read_to_end()` in
//     the current implementation.
//   - super large body provided by user
//   - test without the "Connection: close" header
//   - SSL/TLS verification
//   - redirects (cycles)
//   - chunked encoding issues

#[test]
#[serial]
fn simple_request() {
    const COMPONENT: &str = "http_request_rte_01";
    const COMPONENT_NAME: &str = "test_component";
    const MODULE_NAME: &str = "test_module";

    let temp_dir = TempDir::new().unwrap();
    utils::component::build(COMPONENT, &temp_dir, COMPONENT_NAME)
        .expect("failed to build component");
    let server = utils::http_server::start();
    utils::component::set("http_server", &server.base_url(), &temp_dir).expect("set failed");
    let app_file_name = utils::packaging::package(&temp_dir, COMPONENT_NAME, MODULE_NAME)
        .expect("failed to package app");

    // TODO[RC]: Build hermes just once for all tests
    utils::hermes::build();

    utils::hermes::run_app(&temp_dir, &app_file_name).expect("failed to run hermes app");

    assert!(utils::assert::app_logs_contain(
        &temp_dir,
        "[TEST] got response with request_id"
    ));

    assert!(utils::assert::app_logs_contain(
        &temp_dir,
        utils::http_server::MOCK_CONTENT
    ));

    // Uncomment the line below if you want to inspect the details
    // available in the temp directory.
    //
    // utils::debug_sleep(&temp_dir);
}
