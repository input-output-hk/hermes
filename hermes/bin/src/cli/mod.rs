//! CLI interpreter for the service

mod run;

use clap::Parser;

#[derive(Parser)]
#[clap(rename_all = "kebab-case")]
/// Cli options
pub(crate) enum Cli {
    /// Run the service
    Run(run::Run),
}

impl Cli {
    /// Execute cli commands of the hermes
    #[allow(dead_code)]
    pub(crate) fn exec(self) -> anyhow::Result<()> {
        match self {
            Cli::Run(run) => run.exec(),
        }
    }
}
