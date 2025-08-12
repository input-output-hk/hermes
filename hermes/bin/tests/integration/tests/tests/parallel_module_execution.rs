use std::{fs, io::Write, path::Path, process::Command, time::Instant};

use temp_dir::TempDir;
use uuid::Uuid;

use crate::utils;

fn build_sleep_component(
    component: &str,
    temp_dir: &TempDir,
) -> anyhow::Result<()> {
    let component_path = format!("tests/integration/components/{component}");
    let output = Command::new("cargo")
        .arg("build")
        .arg("--release")
        .arg("--target")
        .arg("wasm32-wasip2")
        .current_dir(&component_path)
        .arg("--target-dir")
        .arg("target")
        .output()?;

    if !output.status.success() {
        anyhow::bail!(
            "cargo build failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let wasm_binary_path =
        format!("{component_path}/target/wasm32-wasip2/release/{component}.wasm");

    let destination_path = temp_dir.as_ref().join(format!("{component}.wasm"));
    std::fs::copy(wasm_binary_path, destination_path)?;

    // Copy settings file
    let settings_file_name = "settings.schema.json";
    let settings_file =
        format!("tests/integration/components/{component}/settings/{settings_file_name}");
    let destination_path = temp_dir.as_ref().join(settings_file_name);

    if Path::new(&settings_file).exists() {
        fs::copy(&settings_file, &destination_path)?;
    } else {
        let mut file = fs::File::create(&destination_path)?;
        file.write_all(b"{}")?;
    }

    Ok(())
}

fn create_multiple_sleep_modules(
    temp_dir: &TempDir,
    server: &str,
) -> anyhow::Result<Vec<String>> {
    // Copy support files
    let support_files = &[
        "config.json",
        "icon.svg",
        "config.schema.json",
        "metadata.json",
    ];

    for &name in support_files {
        let file_path = format!("tests/integration/tests/utils/app_support_files/{name}");
        let destination = temp_dir.as_ref().join(name);
        std::fs::copy(file_path, destination)?;
    }

    // Create 5 separate
    // modules with different
    // settings
    let mut modules = Vec::new();

    for i in 1..=5 {
        let module_name = format!("sleep_module_{i}");

        // Create module
        // manifest
        let module_manifest = format!(
            r#"{{
  "$schema": "https://raw.githubusercontent.com/input-output-hk/hermes/main/hermes/schemas/hermes_module_manifest.schema.json",
  "name": "{module_name}",
  "metadata": "metadata.json",
  "component": "sleep_component.wasm",
  "config": {{
    "file": "config.json",
    "schema": "config.schema.json"
  }},
  "settings": {{
    "file": "settings.schema.json",
    "schema": "settings.schema.json"
  }}
}}"#
        );

        let manifest_path = temp_dir.as_ref().join(format!("manifest_module_{i}.json"));
        fs::write(&manifest_path, module_manifest)?;

        // Create settings
        // file for this
        // module
        let settings_content = format!(
            r#"{{
          "http_server": "{server}"
        }}"#
        );
        let settings_path = temp_dir.as_ref().join("settings.schema.json");
        fs::write(&settings_path, settings_content)?;

        // Package the module
        let output = Command::new(utils::HERMES_BINARY_PATH)
            .arg("module")
            .arg("package")
            .arg(&manifest_path)
            .arg("--output")
            .arg(temp_dir.as_ref())
            .output()?;

        if !output.status.success() {
            anyhow::bail!(
                "module package failed for {}: {}",
                module_name,
                String::from_utf8_lossy(&output.stderr)
            );
        }

        modules.push(format!(
            r#"{{
            "package": "{module_name}.hmod",
            "name": "{module_name}"
        }}"#
        ));
    }

    Ok(modules)
}

fn create_app(
    temp_dir: &TempDir,
    modules: &[String],
) -> anyhow::Result<String> {
    // Create app manifest
    // with all 5 modules
    let app_manifest = format!(
        r#"{{
    "$schema": "https://raw.githubusercontent.com/input-output-hk/hermes/main/hermes/schemas/hermes_app_manifest.schema.json",
    "icon": "icon.svg",
    "metadata": "metadata.json",
    "modules": [
        {}
    ]
}}"#,
        modules.join(",\n        ")
    );
    let app_manifest_path = temp_dir.as_ref().join("manifest_app.json");
    fs::write(&app_manifest_path, app_manifest)?;
    // Package the app
    let app_filename = format!("{}.happ", Uuid::new_v4());
    let output = Command::new(utils::HERMES_BINARY_PATH)
        .arg("app")
        .arg("package")
        .arg("--name")
        .arg(app_filename.clone())
        .arg(&app_manifest_path)
        .output()?;

    if !output.status.success() {
        anyhow::bail!(
            "app package failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    Ok(app_filename)
}

#[test]
#[allow(unreachable_code)]
fn parallel_execution() {
    let temp_dir = TempDir::new().unwrap();

    println!("{}", temp_dir.path().display());
    // Build the component
    // once
    build_sleep_component("sleep_component", &temp_dir).expect("failed to build sleep component");
    println!("finish build");
    let server = utils::http_server::start();
    let modules = create_multiple_sleep_modules(&temp_dir, &server.base_url())
        .expect("failed to package app");
    utils::component::set("http_server", &server.base_url(), &temp_dir).expect("set failed");
    let app_file_name = create_app(&temp_dir, &modules).expect("failed to create app");
    println!("{app_file_name}");
    println!("path: {}/{app_file_name}", temp_dir.path().display());
    println!("waiting for build of hermes app");
    // TODO[RC]: Build hermes
    // just once for all tests
    utils::hermes::build();
    println!("hermes was build");

    // Measure execution time
    // to verify parallel
    // execution
    // #[allow(clippy::panic)]
    println!("running app");

    let start_time = Instant::now();
    utils::hermes::run_app(&temp_dir, &app_file_name).expect("failed to run hermes app");
    let execution_time = start_time.elapsed();
    println!("app finished");

    // Verify all 5 modules
    // started and completed
    for i in 1..=5 {
        assert!(
            utils::assert::app_logs_contain(
                &temp_dir,
                &format!("Module module_{i} starting sleep for 5 seconds")
            ),
            "Module {i} should have started"
        );

        assert!(
            utils::assert::app_logs_contain(
                &temp_dir,
                &format!("Module module_{i} completed sleep after 5 seconds")
            ),
            "Module {i} should have completed"
        );
    }

    // If modules run in
    // parallel, total time
    // should be ~5 seconds,
    // not ~25 seconds
    // Allow some margin for
    // startup/shutdown time
    assert!(
        execution_time.as_secs() < 15,
        "Execution took {} seconds, expected less than 15 seconds for parallel execution",
        execution_time.as_secs()
    );

    println!(
        "Test completed in {} seconds - modules executed in parallel!",
        execution_time.as_secs()
    );

    // Uncomment the line
    // below if you want to
    // inspect the details
    // available in the temp
    // directory.
    //
    // utils::debug_sleep(&
    // temp_dir);
}
