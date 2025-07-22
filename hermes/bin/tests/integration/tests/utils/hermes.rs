use std::{
    process::{Command, Stdio},
    thread,
    time::Duration,
};

use temp_dir::TempDir;

use crate::utils;

const WAIT_TIME: Duration = Duration::from_secs(60 * 2);

pub fn build() {
    println!("BUILDING HERMES");
    let output = Command::new("cargo")
        .arg("build")
        .arg("--manifest-path")
        .arg("/home/magister/IOHK/hermes/hermes/bin/Cargo.toml") // TODO[RC]: Fix hardcoded path
        .output()
        .expect("Failed to execute command");

    if output.status.success() {
        println!("Build succeeded!");
    } else {
        println!("Build failed!");
        println!("Stderr: {}", String::from_utf8_lossy(&output.stderr));
    }

    println!("Build output: {}", String::from_utf8_lossy(&output.stdout));
}

pub fn run_app(temp_dir: &TempDir, app_file_name: &str) -> anyhow::Result<String> {
    let app_path = temp_dir.as_ref().join(app_file_name);

    println!(
        "Running hermes app for {WAIT_TIME:?} seconds: {}",
        app_path.display()
    );

    let mut child = Command::new(utils::HERMES_BINARY_PATH)
        .arg("run")
        .arg("--untrusted")
        .arg(app_path)
        .env("HERMES_LOG_LEVEL", "trace")
        .stdout(Stdio::piped())
        .spawn()?;

    thread::sleep(WAIT_TIME);

    // TODO[RC]: We can dodge the explicit kill by using the exit code RTE.
    match child.kill() {
        Ok(_) => println!("Killed"),
        Err(e) => eprintln!("Failed to kill child process: {}", e),
    }

    let output = child.wait_with_output()?;

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}
