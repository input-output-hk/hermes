//! CLI interpreter for the cbork lint tool

use std::{path::PathBuf, process::exit};

use clap::Parser;
use console::{style, Emoji};

/// CDDL linter cli tool
#[derive(Parser)]
pub(crate) struct Cli {
    /// Path to the CDDL files definition.
    /// It could path to the standalone file, or to the directory.
    path: PathBuf,
}

impl Cli {
    /// Execute the CLI
    pub(crate) fn exec(self) {
        if let Err(err) = check_file(&self.path) {
            println!(
                "{} {}:\n{}",
                Emoji::new("🚨", "Errors"),
                self.path.display(),
                style(err).red()
            );
            exit(1);
        } else {
            println!(
                "{} {}",
                Emoji::new("✅", "Success"),
                self.path.display(),
            );
        }
    }
}

/// Check the CDDL file, return any errors
fn check_file(file_path: &PathBuf) -> anyhow::Result<()> {
    let mut content = std::fs::read_to_string(file_path)?;
    cddl_parser::parse_cddl(&mut content, &cddl_parser::Extension::CDDLParser)
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    Ok(())
}
