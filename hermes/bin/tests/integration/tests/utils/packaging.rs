use std::{env, process::Command};

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
    "settings.schema.json",
];

fn copy_support_files(temp_dir: &TempDir) -> anyhow::Result<()> {
    for &name in SUPPORT_FILES {
        let file_path = format!("tests/integration/tests/utils/app_support_files/{}", name);
        let destination = temp_dir.as_ref().join(name);
        std::fs::copy(file_path, destination)?;
    }
    Ok(())
}

pub fn package_module(temp_dir: &TempDir) -> anyhow::Result<()> {
    copy_support_files(temp_dir)?;

    let manifest_path = temp_dir.as_ref().join("manifest_module.json");

    // TODO[RC]: Double check if failed packaging process really causes an error here.
    let output = Command::new(utils::HERMES_BINARY_PATH)
        .arg("module")
        .arg("package")
        .arg(manifest_path)
        .arg("--output")
        .arg(temp_dir.as_ref())
        .output()?;

    if !output.status.success() {
        return anyhow::bail!(
            "module package failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    Ok(())
}

pub fn package_app(temp_dir: &TempDir) -> anyhow::Result<String> {
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
        return anyhow::bail!(
            "app package failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    Ok(app_filename)
}
