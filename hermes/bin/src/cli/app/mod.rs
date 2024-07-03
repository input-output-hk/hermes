//! cli app command

use clap::Subcommand;

mod package;

/// Hermes cli commands
#[derive(Subcommand)]
pub(crate) enum Commands {
    /// package wasm module
    Package(package::PackageCommand),
}

impl Commands {
    /// Execute cli module command
    pub(crate) fn exec(self) -> anyhow::Result<()> {
        match self {
            Commands::Package(cmd) => cmd.exec(),
        }
    }
}
