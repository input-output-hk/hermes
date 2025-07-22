use std::process::Command;

use temp_dir::TempDir;

use crate::utils;

pub fn package_module(temp_dir: &TempDir) -> anyhow::Result<()> {
    // TODO[RC]: Fix hardcoded manifest
    let name = "config.json";
    let file = format!(
        "/home/magister/IOHK/hermes/hermes/bin/tests/integration/tests/utils/app_support_files/{}",
        name
    );
    std::fs::copy(file, temp_dir.as_ref().join(name))?;
    let name = "icon.svg";
    let file = format!(
        "/home/magister/IOHK/hermes/hermes/bin/tests/integration/tests/utils/app_support_files/{}",
        name
    );
    std::fs::copy(file, temp_dir.as_ref().join(name))?;
    let name = "config.schema.json";
    let file = format!(
        "/home/magister/IOHK/hermes/hermes/bin/tests/integration/tests/utils/app_support_files/{}",
        name
    );
    std::fs::copy(file, temp_dir.as_ref().join(name))?;
    let name = "manifest_module.json";
    let file = format!(
        "/home/magister/IOHK/hermes/hermes/bin/tests/integration/tests/utils/app_support_files/{}",
        name
    );
    std::fs::copy(file, temp_dir.as_ref().join(name))?;
    let name = "manifest_app.json";
    let file = format!(
        "/home/magister/IOHK/hermes/hermes/bin/tests/integration/tests/utils/app_support_files/{}",
        name
    );
    std::fs::copy(file, temp_dir.as_ref().join(name))?;
    let name = "metadata.json";
    let file = format!(
        "/home/magister/IOHK/hermes/hermes/bin/tests/integration/tests/utils/app_support_files/{}",
        name
    );
    std::fs::copy(file, temp_dir.as_ref().join(name))?;
    let name = "settings.schema.json";
    let file = format!(
        "/home/magister/IOHK/hermes/hermes/bin/tests/integration/tests/utils/app_support_files/{}",
        name
    );
    std::fs::copy(file, temp_dir.as_ref().join(name))?;

    let manifest_path = temp_dir.as_ref().join("manifest_module.json");

    println!("PACKAGING MODULE");
    // TODO[RC]: Double check if failed packaging process really causes an error here.
    let output = Command::new(utils::HERMES_BINARY_PATH)
        .arg("module")
        .arg("package")
        .arg(manifest_path)
        .arg("--output")
        .arg(temp_dir.as_ref())
        .output()?;

    println!("output: {}", String::from_utf8_lossy(&output.stdout));

    if !output.status.success() {
        return anyhow::bail!(
            "module package failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    Ok(())
}

pub fn package_app(temp_dir: &TempDir) -> anyhow::Result<()> {
    let manifest_path = temp_dir.as_ref().join("manifest_app.json");

    println!("PACKAGING APP");
    let output = Command::new(utils::HERMES_BINARY_PATH)
        .arg("app")
        .arg("package")
        .arg(manifest_path)
        .output()?;

    println!("output: {}", String::from_utf8_lossy(&output.stdout));

    if !output.status.success() {
        return anyhow::bail!(
            "app package failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    Ok(())
}
