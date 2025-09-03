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
    const EVENT_COUNT: usize = 5;
    const TIME_IN_SECS_PER_EVENT: usize = 5;
    const BUFFER_SECS: usize = 15;
    const EXPECTED_EXECUTION_TIME_IN_SECONDS: usize = TIME_IN_SECS_PER_EVENT + BUFFER_SECS;
    const RESERVED_THREADS_FOR_TASK_QUEUE: usize = 1;
    const RESERVED_THREAD_FOR_MAIN: usize = 1;
    const REQUIRED_THREAD_COUNT: usize =
        EVENT_COUNT + RESERVED_THREADS_FOR_TASK_QUEUE + RESERVED_THREAD_FOR_MAIN;

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
    for i in 0..EVENT_COUNT {
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
        &format!("All {EVENT_COUNT} responses written correctly, calling done()")
    ));

    // If events run in parallel, total time should be ~`TIME_SECS_PER_EVENT` seconds, not
    // ~`TIME_SECS_PER_EVENT` * `EVENT_COUNT` seconds Allow some margin for
    // startup/shutdown time and database contention
    //
    // Note: if there is not enough threads, then we would have some kind of sequential
    // execution, so this assert would not pass
    // We need `EVENT_COUNT` threads for all workers to run in parallel,
    // `RESERVED_THREADS_FOR_TASK_QUEUE` thread(s) for the task queue,
    // and we also count the `RESERVED_THREAD_FOR_MAIN` itself since it participates
    // in Rayon’s work stealing.
    if available_parallelism()
        .expect("could not check available number of threads")
        .get()
        > REQUIRED_THREAD_COUNT
    {
        assert!(
            execution_time.as_secs() < EXPECTED_EXECUTION_TIME_IN_SECONDS as u64,
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
