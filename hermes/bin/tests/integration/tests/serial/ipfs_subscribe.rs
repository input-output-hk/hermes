use serial_test::serial;
use temp_dir::TempDir;

use crate::utils;

#[test]
#[serial]
fn ipfs_subscribe() {
    const COMPONENT: &str = "ipfs_subscribe";
    const MODULE: &str = "ipfs_subscribe_module";

    let temp_dir = TempDir::new().unwrap();
    utils::component::build(COMPONENT, &temp_dir).expect("failed to build component");

    let app_file_name =
        utils::packaging::package(&temp_dir, COMPONENT, MODULE).expect("failed to package app");

    // TODO[RC]: Build hermes just once for all tests
    utils::hermes::build();

    utils::hermes::run_app(&temp_dir, &app_file_name).expect_err("should fail to run hermes app");

    assert!(utils::assert::app_logs_contain(
        &temp_dir,
        "added subscription topic stream"
    ));

    assert!(utils::assert::app_logs_contain(
        &temp_dir,
        "\"pubsub_topic\":\"ipfs_channel\""
    ));

    // Uncomment the line below if you want to inspect the details
    // available in the temp directory.
    //
    // utils::debug_sleep(&temp_dir);
}
