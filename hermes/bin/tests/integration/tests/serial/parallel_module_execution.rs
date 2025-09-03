use std::{thread::available_parallelism, time::Instant};

use serial_test::serial;
use temp_dir::TempDir;

use crate::utils;

#[test]
#[serial]
fn parallel_execution() {
    const COMPONENT: &str = "sleep_component";
    const COMPONENT_NAME: &str = "sleep_component";
    const MODULE_NAME: &str = "sleep_module";
    const EXPECTED_EXECUTION_TIME_IN_SECONDS: u64 = 20;

    let temp_dir = TempDir::new().unwrap();
    utils::component::build(COMPONENT, &temp_dir, COMPONENT_NAME)
        .expect("failed to build component");
    let server = utils::http_server::start();
    utils::component::set("http_server", &server.base_url(), &temp_dir).expect("set failed");
    let app_file_name = utils::packaging::package(&temp_dir, COMPONENT_NAME, MODULE_NAME)
        .expect("failed to package app");

    // TODO[RC]: Build hermes just once for all tests
    utils::hermes::build();

    // Measure execution time to verify parallel execution
    let start_time = Instant::now();
    utils::hermes::run_app(&temp_dir, &app_file_name).expect("failed to run hermes app");
    let execution_time = start_time.elapsed();

    // Check if initialization started
    assert!(utils::assert::app_logs_contain(
        &temp_dir,
        "init sleep component"
    ));

    // Verify all events started and completed
    for i in 0..5 {
        assert!(utils::assert::app_logs_contain(
            &temp_dir,
            &format!("sending sleep app request {i}")
        ));

        assert!(utils::assert::app_logs_contain(
            &temp_dir,
            &format!("got response with request_id={:?}", Some(i))
        ));
    }

    assert!(utils::assert::app_logs_contain(
        &temp_dir,
        &format!("All {} responses written correctly, calling done()", 5)
    ));

    // If events run in parallel, total time should be ~5 seconds, not ~25 seconds
    // Allow some margin for startup/shutdown time and database contention
    //
    // Note: if there is not enough threads, then we would have some kind of sequential
    // execution, so this assert would not pass
    // We need 1 thread for task queue, 1 thread for thread pool and
    // 5 for each worker to run independently
    if available_parallelism()
        .expect("could not check available number of threads")
        .get()
        > 6
    {
        assert!(
            execution_time.as_secs() < EXPECTED_EXECUTION_TIME_IN_SECONDS,
            "Execution took {} seconds, expected less than {} seconds for parallel execution",
            execution_time.as_secs(),
            EXPECTED_EXECUTION_TIME_IN_SECONDS
        );
    }

    // Uncomment the line below if you want to inspect the details
    // available in the temp directory.
    //
    // utils::debug_sleep(&temp_dir);
}
