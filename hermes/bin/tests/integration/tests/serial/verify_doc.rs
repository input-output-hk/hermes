use serial_test::serial;
use temp_dir::TempDir;

use crate::utils;

#[test]
#[serial]
fn verify_doc_runs() {
    const COMPONENT: &str = "verify_doc";
    const MODULE: &str = "test_module";

    let temp_dir = TempDir::new().unwrap().dont_delete_on_drop();

    utils::component::build(COMPONENT, &temp_dir).expect("failed to build component");
    let app_file_name =
        utils::packaging::package(&temp_dir, COMPONENT, MODULE).expect("failed to package app");

    // TODO[RC]: Build hermes just once for all tests
    utils::hermes::build();

    // TODO[RC]: Currently, when a module fails to initialize, the whole app fails to run.
    // In future, hermes will not bail on app initialization.
    utils::hermes::run_app(&temp_dir, &app_file_name).expect_err("should fail to run hermes app");

    // Uncomment the line below if you want to inspect the details
    // available in the temp directory.
    //
    // utils::debug_sleep(&temp_dir);
}
