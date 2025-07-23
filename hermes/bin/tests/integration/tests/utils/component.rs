use std::{fs, io::Write, path::Path, process::Command};

use temp_dir::TempDir;

const SETTINGS_FILE_NAME: &str = "settings.json";

fn copy_settings_file(component: &str, temp_dir: &TempDir) -> anyhow::Result<()> {
    let settings_file = format!(
        "tests/integration/components/{}/settings/{}",
        component, SETTINGS_FILE_NAME
    );
    let destination_path = temp_dir.as_ref().join(SETTINGS_FILE_NAME);
    if Path::new(&settings_file).exists() {
        fs::copy(&settings_file, &destination_path)?;
    } else {
        let mut file = fs::File::create(&destination_path)?;
        file.write_all(b"{}")?;
    }
    Ok(())
}

pub fn set(key: &str, value: &str, temp_dir: &TempDir) -> anyhow::Result<()> {
    let settings_file = temp_dir.as_ref().join(SETTINGS_FILE_NAME);

    let placeholder = format!("{{{{{}}}}}", key);
    let settings = fs::read_to_string(&settings_file)?;

    let settings = settings.replace(&placeholder, value);
    fs::write(settings_file, settings)?;
    Ok(())
}

pub fn build(component: &str, temp_dir: &TempDir) -> anyhow::Result<()> {
    let component_path = format!("tests/integration/components/{}", component);
    let output = Command::new("cargo")
        .arg("build")
        .arg("--release")
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
        "{}/target/wasm32-wasip2/release/test_component.wasm",
        component_path
    );

    let destination_path = temp_dir.as_ref().join("test_component.wasm");
    std::fs::copy(wasm_binary_path, destination_path)?;

    copy_settings_file(component, temp_dir)
}
