use std::process::Command;

use temp_dir::TempDir;

pub fn build(component: &str, temp_dir: &TempDir) -> anyhow::Result<()> {
    // TODO[RC]: Fix hardcoded path
    let component_path = format!(
        "/home/magister/IOHK/hermes/hermes/bin/tests/integration/components/{}",
        component
    );
    let output = Command::new("cargo")
        .arg("build")
        .arg("--target")
        .arg("wasm32-wasip2")
        .current_dir(&component_path)
        .output()?;

    if !output.status.success() {
        return anyhow::bail!(
            "cargo build failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let wasm_binary_path = format!(
        "{}/target/wasm32-wasip2/debug/test_component.wasm",
        component_path
    );

    let destination_path = temp_dir.as_ref().join("test_component.wasm");
    std::fs::copy(wasm_binary_path, destination_path)?;

    Ok(())
}
