use std::process::Command;

pub fn build(component: &str) -> anyhow::Result<()> {
    let component_path = format!(
        "/home/magister/IOHK/hermes/hermes/bin/tests/integration/components/{}",
        component
    );
    let output = Command::new("cargo")
        .arg("build")
        .arg("--target")
        .arg("wasm32-wasip2")
        .current_dir(component_path)
        .output()
        .expect("Failed to execute cargo build");

    if !output.status.success() {
        return anyhow::bail!(
            "cargo build failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}
