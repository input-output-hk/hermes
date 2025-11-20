use std::{
    fs::File,
    process::{Command, Stdio},
};

use temp_dir::TempDir;

use crate::utils::{self, LOG_FILE_NAME};

pub fn build() {
    #[cfg(debug_assertions)]
    let output = Command::new("cargo")
        .arg("build")
        .output()
        .expect("Failed to execute command");

    #[cfg(not(debug_assertions))]
    let output = Command::new("cargo")
        .arg("build")
        .arg("--release")
        .output()
        .expect("Failed to execute command");

    if !output.status.success() {
        println!("Build failed!");
        println!("Stderr: {}", String::from_utf8_lossy(&output.stderr));
    }
}

pub fn run_app(
    temp_dir: &TempDir,
    app_name: &str,
) -> anyhow::Result<String> {
    let mut app_path = temp_dir.as_ref().join(app_name);
    app_path.set_extension("happ");

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
        return Err(anyhow::anyhow!("App failed with error: {stderr}"));
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// Spawn Hermes as a child process and return a guard that will
/// terminate it on drop.
pub fn spawn_app(
    temp_dir: &TempDir,
    app_name: &str,
) -> anyhow::Result<HermesChild> {
    let mut app_path = temp_dir.as_ref().join(app_name);
    app_path.set_extension("happ");

    let log_file_path = temp_dir.as_ref().join(LOG_FILE_NAME);
    let log_file = File::create(&log_file_path)?;

    let child = Command::new(utils::HERMES_BINARY_PATH)
        .arg("run")
        .arg("--untrusted")
        .arg(app_path)
        .env("HERMES_LOG_LEVEL", "trace")
        // Disable auth for integration test
        .env("HERMES_ACTIVATE_AUTH", "false")
        .stdout(Stdio::from(log_file.try_clone()?))
        .stderr(Stdio::from(log_file))
        .spawn()?;

    Ok(HermesChild { child })
}

pub struct HermesChild {
    child: std::process::Child,
}

impl HermesChild {
    /// Try to determine if child is still running.
    pub fn is_running(&mut self) -> bool {
        matches!(self.child.try_wait(), Ok(None))
    }
}

impl Drop for HermesChild {
    fn drop(&mut self) {
        // If still running, try to kill and wait a bit
        if self.is_running() {
            self.child.kill().ok();
            // Best-effort wait
            for _ in 0..10 {
                if let Ok(Some(_)) = self.child.try_wait() {
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
            self.child.wait().ok();
        }
    }
}
