use std::{
    fs::File,
    process::{Command, Stdio},
};

use temp_dir::TempDir;

use crate::utils::{self, LOG_FILE_NAME};

pub fn build() {
    let output = Command::new("cargo")
        .arg("build")
        .arg("--release") // TODO[RC]: This should respect the proper build profile.
        .output()
        .expect("Failed to execute command");

    if !output.status.success() {
        println!("Build failed!");
        println!("Stderr: {}", String::from_utf8_lossy(&output.stderr));
    }
}

pub fn run_app(
    temp_dir: &TempDir,
    app_file_name: &str,
) -> anyhow::Result<String> {
    let app_path = temp_dir.as_ref().join(app_file_name);

    let log_file_path = temp_dir.as_ref().join(LOG_FILE_NAME);
    let log_file = File::create(&log_file_path)?;

    let child = Command::new(utils::HERMES_BINARY_PATH)
        .arg("run")
        .arg("--untrusted")
        .arg(app_path)
        .env("HERMES_LOG_LEVEL", "trace")
        .stdout(Stdio::from(log_file.try_clone()?))
        .stderr(Stdio::from(log_file))
        .spawn()?;

    let output = child.wait_with_output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow::anyhow!("App failed with error: {}", stderr));
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}
