//! cli app command

use clap::Subcommand;

mod package;
mod sign;

/// Hermes cli app commands
#[derive(Subcommand)]
pub(crate) enum Commands {
    /// package application
    Package(package::PackageCommand),
    /// sign application
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
