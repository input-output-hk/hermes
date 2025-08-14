use std::process::Command;

use temp_dir::TempDir;
use uuid::Uuid;

use crate::utils;

const SUPPORT_FILES: &[&str] = &[
    "config.json",
    "icon.svg",
    "config.schema.json",
    "manifest_module.json",
    "manifest_app.json",
    "metadata.json",
];

fn copy_support_files(
    temp_dir: &TempDir,
    component_name: &str,
    module_name: &str,
) -> anyhow::Result<()> {
    for &name in SUPPORT_FILES {
        let file_path = format!("tests/integration/tests/utils/app_support_files/{name}");
        let destination = temp_dir.as_ref().join(name);
        let mut file_content = std::fs::read_to_string(file_path)?;
        file_content = file_content.replace("test_module", module_name);
        file_content = file_content.replace("test_component", component_name);
        std::fs::write(destination, file_content)?;
    }
    Ok(())
}

pub fn package(
    temp_dir: &TempDir,
    component_name: &str,
    module_name: &str,
) -> anyhow::Result<String> {
    package_module(temp_dir, component_name, module_name)?;
    package_app(temp_dir)
}

fn package_module(
    temp_dir: &TempDir,
    component_name: &str,
    module_name: &str,
) -> anyhow::Result<()> {
    copy_support_files(temp_dir, component_name, module_name)?;

    let manifest_path = temp_dir.as_ref().join("manifest_module.json");
    let output = Command::new(utils::HERMES_BINARY_PATH)
        .arg("module")
        .arg("package")
        .arg(manifest_path)
        .arg("--output")
        .arg(temp_dir.as_ref())
        .output()?;

    if !output.status.success() {
        anyhow::bail!(
            "module package failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    Ok(())
}

fn package_app(temp_dir: &TempDir) -> anyhow::Result<String> {
    let manifest_path = temp_dir.as_ref().join("manifest_app.json");
    let app_filename = format!("{}.happ", Uuid::new_v4());

    let output = Command::new(utils::HERMES_BINARY_PATH)
        .arg("app")
        .arg("package")
        .arg("--name")
        .arg(app_filename.clone())
        .arg(manifest_path)
        .output()?;

    if !output.status.success() {
        anyhow::bail!(
            "app package failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    Ok(app_filename)
}
