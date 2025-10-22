use hyper::Method;

use crate::serial::athena::build::build_athena;

#[tokio::test]
async fn check_empty_db_request() {
    let app_file_name = build_athena().expect("failed to build athena app");

    utils::hermes::build();

    let handler = tokio::spawn(
        utils::hermes::run_app(&temp_dir, &app_file_name)
            .expect_err("should fail to run hermes app"),
    );

    let req = reqwest::Request::new(Method::GET, "");
}
