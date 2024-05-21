//! CLI interpreter for the service

mod build_info;
mod run;

use build_info::BUILD_INFO;
use clap::{Parser, Subcommand};

/// Hermes
///
/// Hermes node application which could be used to run a hermes node itself by executing
/// just `./hermes` without any arguments.
/// And also it could be used to package, sign, verify and distribute hermes apps using
/// corresponding commands.
#[derive(Parser)]
#[clap(version = BUILD_INFO)]
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
    pub(crate) fn exec(self) -> anyhow::Result<()> {
        println!("{BUILD_INFO}");
        match self.command {
            None => run::Run::exec(),
            Some(Commands::Package) => todo!(),
        }
    }
}
