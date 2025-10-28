use anyhow::Context;
use reqwest::{
    header::{HeaderName, HeaderValue, AUTHORIZATION, CONTENT_TYPE, HOST},
    StatusCode,
};
use serial_test::serial;
use std::{path::Path, time::Duration};
use std::{str::FromStr, sync::Arc};
use temp_dir::TempDir;

use crate::utils;

fn build_stake_ada_with_db_mock(temp_dir: &TempDir) -> anyhow::Result<String> {
    const COMPONENT: &str = "staked-ada";
    const MOCK_COMPONENT: &str = "staked_ada_indexer_mock";

    let manifest_dir_path = Path::new(env!("CARGO_MANIFEST_DIR"));
    let hermes_root = manifest_dir_path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("could not find parent directory for bin"))?;
    let athena_modules_path = hermes_root.join("apps/athena/modules");
    utils::component::build_at_path(&athena_modules_path, COMPONENT, temp_dir)
        .map_err(|err| anyhow::anyhow!("failed to build {COMPONENT} component: {err}"))?;
    utils::component::build(MOCK_COMPONENT, temp_dir).context("failed to build component")?;
    let components = ["staked_ada_indexer_mock", "staked_ada"];
    for component in &components {
        println!("packaging {component} module for component {component}");
        utils::packaging::package_module(temp_dir, component, component)
            .map_err(|err| anyhow::anyhow!("failed to package {component} module: {err}"))?;
    }

    let modules = components
        .into_iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>();

    let app_name = utils::packaging::package_app_with_modules(temp_dir, Some(modules))
        .map_err(|err| anyhow::anyhow!("failed to package athena app: {err}"))?;

    Ok(app_name)
}

macro_rules! ensure_eq {
    ($left:expr, $right:expr) => {
        match (&$left, &$right) {
            (left_val, right_val) => {
                if !(*left_val == *right_val) {
                    anyhow::bail!(
                        "assertion failed: `(left == right)`\n  left: `{:?}`,\n right: `{:?}`",
                        left_val,
                        right_val
                    )
                }
            }
        }
    };
    ($left:expr, $right:expr, $($arg:tt)+) => {
        match (&$left, &$right) {
            (left_val, right_val) => {
                if !(*left_val == *right_val) {
                    anyhow::bail!(
                        "assertion failed: `(left == right)`\n  left: `{:?}`,\n right: `{:?}`: {}",
                        left_val,
                        right_val,
                        format!($($arg)+)
                    )
                }
            }
        }
    };
}

fn build_request_for_athena(
    url: &str,
    app_name: &str,
    client: &reqwest::blocking::Client,
) -> anyhow::Result<reqwest::blocking::Request> {
    let request = client
        .get(url)
        .headers(
            [
                (
                    HOST,
                    HeaderValue::from_str(&format!("{app_name}.hermes.local"))
                        .context("invalid header value")?,
                ),
                (CONTENT_TYPE, HeaderValue::from_static("application/json")),
                (
                    AUTHORIZATION,
                    HeaderValue::from_static("Bearer your-token-here"),
                ),
                (
                    HeaderName::from_lowercase(b"x-custom-header")
                        .context("invalid header name")?,
                    HeaderValue::from_static("custom-value"),
                ),
            ]
            .into_iter()
            .collect(),
        )
        .build()?;
    Ok(request)
}

struct StakedData<'a> {
    ada_amount: &'a serde_json::Number,
    slot_number: u64,
    assets: &'a Vec<serde_json::Value>,
}

fn extract_staked_data_from_json(data: &serde_json::Value) -> anyhow::Result<StakedData<'_>> {
    Ok(StakedData {
        ada_amount: data
            .get("ada_amount")
            .context("ada_amount is missing")
            .and_then(|ada_amount| {
                serde_json::Value::as_number(ada_amount).context("ada_amount is not Number")
            })?,
        slot_number: data
            .get("slot_number")
            .context("slot_number is missing")
            .and_then(|slot_number| {
                serde_json::Value::as_u64(slot_number).context("slot_number is not u64")
            })?,
        assets: data
            .get("assets")
            .context("assets are missing")
            .and_then(|assets| {
                serde_json::Value::as_array(assets).context("assets is not and array")
            })?,
    })
}

#[test]
#[serial]
fn staked_ada_requests() -> anyhow::Result<()> {
    let temp_dir = Arc::new(TempDir::new()?.dont_delete_on_drop());

    let app_name = build_stake_ada_with_db_mock(&temp_dir).context("failed to build modules")?;

    // TODO[RC]: Build hermes just once for all tests
    utils::hermes::build();
    let handler = std::thread::spawn({
        let temp_dir = temp_dir.clone();
        let app_name = app_name.clone();
        move || {
            println!("Running application: {app_name}");
            utils::hermes::run_app(&temp_dir, &app_name)?;
            println!("Application run terminated");
            Ok::<(), anyhow::Error>(())
        }
    });

    // Wait for app to initialize modules.
    std::thread::sleep(Duration::from_secs(10));

    let url_builder = |stake_address: &str| {
        format!("http://localhost:5000/api/gateway/v1/cardano/assets/{stake_address}")
    };
    let url_with_mocked_address =
        url_builder("stake1ux5wm486ud2racwpyrnngpzvjfcf839dacpvd60djgfkd0cfzwyau");
    let url_with_unknown_address =
        url_builder("stake1u9658xgzll2su0mpfgjykz86zutmv7x737vcdgf3nsu03wqt63ggw");
    let url_with_invalid_address = url_builder("blah_blah");

    let client = reqwest::blocking::Client::new();

    //
    //
    //                 Mocked stake address
    //
    //
    {
        let request_for_mocked_address =
            build_request_for_athena(&url_with_mocked_address, &app_name, &client)
                .context("failed to build request")?;
        let response_for_mocked_address = client
            .execute(request_for_mocked_address)
            .context("failed to get response from Athena")?;
        let status = response_for_mocked_address.status();
        ensure_eq!(status, StatusCode::OK, "Expected 200 OK but got {status:?}");

        let response_body = response_for_mocked_address
            .text()
            .context("failed to get response body")?;

        let response_data: serde_json::Value =
            serde_json::from_str(&response_body).context("failed to parse response as JSON")?;

        let (persistent_data, volatile_data) = (
            response_data
                .get("persistent")
                .context("persistent should exist")?,
            response_data
                .get("volatile")
                .context("volatile should exist")?,
        );
        let persistent_data = extract_staked_data_from_json(persistent_data)?;
        ensure_eq!(
            persistent_data.ada_amount,
            &serde_json::Number::from_str("100000000")?
        );
        ensure_eq!(
            persistent_data.slot_number,
            12345,
            "Expected slot number to be 12345"
        );
        anyhow::ensure!(persistent_data.assets.is_empty());

        let volatile_data = extract_staked_data_from_json(volatile_data)?;
        ensure_eq!(
            volatile_data.ada_amount,
            &serde_json::Number::from_str("100000000")?
        );
        ensure_eq!(
            volatile_data.slot_number,
            12345,
            "Expected slot number to be 12345"
        );
        anyhow::ensure!(volatile_data.assets.is_empty());
    }

    //
    //
    //                 Unknown stake address
    //
    //
    {
        let request_for_unknown_address =
            build_request_for_athena(&url_with_unknown_address, &app_name, &client)
                .context("failed to build request")?;
        let response_unknown_address = client
            .execute(request_for_unknown_address)
            .context("failed to get response from Athena")?;
        ensure_eq!(response_unknown_address.status(), StatusCode::NOT_FOUND);
    }

    //
    //
    //                 Invalid url
    //
    //
    {
        let request_for_invalid_address =
            build_request_for_athena(&url_with_invalid_address, &app_name, &client)
                .context("failed to build request")?;
        let response_for_invalid_address = client
            .execute(request_for_invalid_address)
            .context("failed to get response from Athena")?;
        ensure_eq!(response_for_invalid_address.status(), StatusCode::NOT_FOUND);
    }

    // Check if app is not terminated
    anyhow::ensure!(!handler.is_finished());

    // Uncomment the line below if you want to inspect the details
    // available in the temp directory.
    //
    // utils::debug_sleep(&temp_dir);
    Ok(())
}
