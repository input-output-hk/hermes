//! CLI interpreter for the cbork lint tool

use std::{path::PathBuf, process::exit};

use clap::Parser;
use console::{style, Emoji};

/// CDDL linter cli tool
#[derive(Parser)]
pub(crate) struct Cli {
    /// Path to the CDDL files definition.
    /// It could path to the standalone file, or to the directory.
    /// So all files with the `.cddl` extension inside the directory will be linted.
    path: PathBuf,
}

impl Cli {
    /// Execute the CLI
    pub(crate) fn exec(self) {
        let res = if self.path.is_file() {
            check_file_with_print(&self.path)
        } else {
            check_dir_with_print(&self.path)
        };

        if !res {
            exit(1);
        }
    }
}

/// Check the CDDL file, return any errors
fn check_file(file_path: &PathBuf) -> anyhow::Result<()> {
    let mut content = std::fs::read_to_string(file_path)?;
    cddl_parser::parse_cddl(&mut content, &cddl_parser::Extension::CDDLParser)?;
    Ok(())
}

/// Check the CDDL file, prints any errors into the stdout
fn check_file_with_print(file_path: &PathBuf) -> bool {
    if let Err(e) = check_file(file_path) {
        println!(
            "{} {}:\n{}",
            Emoji::new("🚨", "Errors"),
            file_path.display(),
            style(e).red()
        );
        false
    } else {
        println!("{} {}", Emoji::new("✅", "Success"), file_path.display(),);
        true
    }
}

/// CDDL file extension. Filter directory files to apply the linter only on the CDDL
/// files.
const CDDL_FILE_EXTENSION: &str = "cddl";

/// Check the directory, prints any errors into the stdout
fn check_dir_with_print(dir_path: &PathBuf) -> bool {
    let fun = |dir_path| -> anyhow::Result<bool> {
        let mut res = true;
        for entry in std::fs::read_dir(dir_path)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() {
                if path.extension().is_some_and(|e| e.eq(CDDL_FILE_EXTENSION)) {
                    res = check_file_with_print(&path);
                }
            } else if path.is_dir() {
                res = check_dir_with_print(&path);
            }
        }
        Ok(res)
    };

    if let Err(e) = fun(dir_path) {
        println!(
            "{} {}:\n{}",
            Emoji::new("🚨", "Errors"),
            dir_path.display(),
            style(e).red()
        );
        false
    } else {
        true
    }
}
