use std::process::Command;

use serde::{Deserialize, Serialize};
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

#[derive(Debug, Serialize, Deserialize, Clone)]
struct ModuleEntry {
    package: String,
    name: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct AppManifest {
    #[serde(rename = "$schema")]
    schema: String,
    icon: String,
    metadata: String,
    pub(crate) modules: Vec<ModuleEntry>,
}

fn replace_app_manifest_with_modules(
    temp_dir: &TempDir,
    modules: &[String],
) -> anyhow::Result<()> {
    let file_path = temp_dir.as_ref().join("manifest_app.json");
    let manifest_content = std::fs::read_to_string(file_path.clone())?;
    let mut app_manifest: AppManifest = serde_json::from_str(&manifest_content)?;
    let new_modules: Vec<ModuleEntry> = modules
        .iter()
        .map(|module| {
            ModuleEntry {
                package: format!("{module}.hmod"),
                name: module.to_string(),
            }
        })
        .collect();
    app_manifest.modules = new_modules;
    let updated_content = serde_json::to_string_pretty(&app_manifest)?;
    Ok(std::fs::write(file_path, updated_content)?)
}

pub fn package(
    temp_dir: &TempDir,
    component_name: &str,
    module_name: &str,
) -> anyhow::Result<String> {
    package_module(temp_dir, component_name, module_name)?;
    package_app(temp_dir)
}

pub fn package_module(
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

pub fn package_app(temp_dir: &TempDir) -> anyhow::Result<String> {
    package_app_with_modules(temp_dir, None)
}

pub fn package_app_with_modules(
    temp_dir: &TempDir,
    modules: Option<Vec<String>>,
) -> anyhow::Result<String> {
    let manifest_path = temp_dir.as_ref().join("manifest_app.json");
    let app_name = Uuid::new_v4().to_string();

    if let Some(modules) = modules {
        replace_app_manifest_with_modules(temp_dir, &modules)?;
    }

    let output = Command::new(utils::HERMES_BINARY_PATH)
        .arg("app")
        .arg("package")
        .arg("--name")
        .arg(&app_name)
        .arg(manifest_path)
        .output()?;

    if !output.status.success() {
        anyhow::bail!(
            "app package failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    Ok(app_name)
}
