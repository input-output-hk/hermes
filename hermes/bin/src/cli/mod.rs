//! CLI interpreter for the service

mod build_info;
mod package;
mod run;

use build_info::BUILD_INFO;
use clap::{Parser, Subcommand};
use console::{style, Emoji};

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
#[derive(Subcommand)]
enum Commands {
    /// Package the app
    Package(package::PackageCommand),
}

impl Cli {
    /// Execute cli commands of the hermes
    #[allow(dead_code)]
    pub(crate) fn exec(self) {
        println!("{BUILD_INFO}");
        if let Err(err) = match self.command {
            None => run::Run::exec(),
            Some(Commands::Package(cmd)) => cmd.exec(),
        } {
            let alarm_emoji = Emoji::new("🚨", "Errors");
            let err_msg = style(err.to_string()).red();
            println!("{alarm_emoji}:\n{err_msg}");
            std::process::exit(1);
        }
    }
}
