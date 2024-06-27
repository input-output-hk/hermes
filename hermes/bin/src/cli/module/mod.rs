//! module cli command

use clap::Subcommand;

mod package;
mod sign;

/// Hermes cli commands
#[derive(Subcommand)]
pub(crate) enum Commands {
    /// package wasm module
    Package(package::PackageCommand),
    /// sign wasm module package
    Sign(sign::SignCommand),
}

impl Commands {
    /// Execute cli module command
    pub(crate) fn exec(self) -> anyhow::Result<()> {
        match self {
            Commands::Package(cmd) => cmd.exec(),
            Commands::Sign(cmd) => cmd.exec(),
        }
    }
}
