use std::process::Command;

use temp_dir::TempDir;

use crate::utils;

pub fn package_module(temp_dir: &TempDir) -> anyhow::Result<()> {
    // TODO[RC]: Fix hardcoded manifest
    let name = "config.json";
    let file = format!("/home/magister/IOHK/hermes/hermes/bin/tests/integration/tests/utils/app_support_files/{}", name);;
    std::fs::copy(file, temp_dir.as_ref().join(name))?;
    let name = "config.schema.json";
    let file = format!("/home/magister/IOHK/hermes/hermes/bin/tests/integration/tests/utils/app_support_files/{}", name);;
    std::fs::copy(file, temp_dir.as_ref().join(name))?;
    let name = "manifest_module.json";
    let file = format!("/home/magister/IOHK/hermes/hermes/bin/tests/integration/tests/utils/app_support_files/{}", name);;
    std::fs::copy(file, temp_dir.as_ref().join(name))?;
    let name = "metadata.json";
    let file = format!("/home/magister/IOHK/hermes/hermes/bin/tests/integration/tests/utils/app_support_files/{}", name);;
    std::fs::copy(file, temp_dir.as_ref().join(name))?;
    let name = "settings.schema.json";
    let file = format!("/home/magister/IOHK/hermes/hermes/bin/tests/integration/tests/utils/app_support_files/{}", name);;
    std::fs::copy(file, temp_dir.as_ref().join(name))?;

    let manifest_path = temp_dir.as_ref().join("manifest_module.json");

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

    println!("output: {}", String::from_utf8_lossy(&output.stdout));

    Ok(())
}

pub fn package_app() -> anyhow::Result<()> {
    // let module_path =
    // "/home/magister/IOHK/hermes/hermes/bin/tests/integration/components/
    // http_request_rte_01/target/wasm32-wasip2/debug/test_component.wasm";

    // let output = Command::new(utils::HERMES_BINARY_PATH)
    //     .arg("module")
    //     .arg("package")
    //     .arg("/home/magister/IOHK/hermes-modules/hello-world-module/manifest_module.json")
    //     .output()?;

    // if !output.status.success() {
    //     return anyhow::bail!(
    //         "module package failed: {}",
    //         String::from_utf8_lossy(&output.stderr)
    //     );
    // }
    Ok(())
}
