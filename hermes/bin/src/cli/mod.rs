//! CLI interpreter for the service

mod run;

use clap::{Parser, Subcommand};

/// Hermes cli
#[derive(Parser)]
#[clap(version, about)]
pub(crate) struct Cli {
    /// Hermes cli subcommand
    #[clap(subcommand)]
    command: Option<Commands>,
}

/// Hermes cli commands
#[derive(Subcommand, Clone)]
enum Commands {
    /// Package the app
    Package,
}

impl Cli {
    /// Execute cli commands of the hermes
    #[allow(dead_code)]
    pub(crate) fn exec(self) -> anyhow::Result<()> {
        match self.command {
            None => run::Run::exec(),
            Some(Commands::Package) => todo!(),
        }
    }
}
