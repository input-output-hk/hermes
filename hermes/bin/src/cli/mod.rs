//! CLI interpreter for the service

mod app;
mod build_info;
mod module;
mod run;

use std::path::PathBuf;

use build_info::BUILD_INFO;
use clap::{Parser, Subcommand};
use console::{style, Emoji};

use crate::{
    errors::Errors,
    logger::{self, LoggerConfigBuilder},
};

/// A parameter identifier specifying the log level.
const ENV_LOG_LEVEL: &str = "HERMES_LOG_LEVEL";

/// Hermes
///
/// Hermes node application which could be used to run a hermes node itself by executing
/// just `./hermes` without any arguments.
/// And also it could be used to package, sign, verify and distribute hermes apps using
/// corresponding commands.
#[derive(Parser)]
#[clap(version = BUILD_INFO)]
pub(crate) struct Cli {
    /// Path to the Hermes application package to run
    app_package: PathBuf,

    /// Path to the trusted certificate
    #[clap(name = "cert", short)]
    certificate: Vec<PathBuf>,

    /// Flag which disables package signature verification
    #[clap(long, action = clap::ArgAction::SetTrue)]
    untrusted: bool,

    /// Hermes cli subcommand
    #[clap(subcommand)]
    command: Option<Commands>,
}

/// Hermes cli commands
#[derive(Subcommand)]
enum Commands {
    /// module commands
    #[command(subcommand)]
    Module(module::Commands),
    /// app commands
    #[command(subcommand)]
    App(app::Commands),
}

impl Cli {
    /// Hermes home directory
    pub(crate) fn hermes_home() -> PathBuf {
        dirs::home_dir()
            .unwrap_or("/var/lib".into())
            .join(".hermes")
    }

    /// Execute cli commands of the hermes
    pub(crate) fn exec(self) {
        println!("{}{}", Emoji::new("ℹ️", ""), style(BUILD_INFO).yellow());

        let mut errors = Errors::new();
        let log_level = std::env::var(ENV_LOG_LEVEL)
            .unwrap_or_default()
            .parse()
            .unwrap_or_default();

        let log_config = LoggerConfigBuilder::default()
            .log_level(log_level)
            .with_thread(true)
            .with_file(true)
            .with_line_num(true)
            .build();

        logger::init(&log_config).unwrap_or_else(errors.get_add_err_fn());

        match self.command {
            None => run::Run::exec(self.app_package, self.certificate, self.untrusted),
            Some(Commands::Module(cmd)) => cmd.exec(),
            Some(Commands::App(cmd)) => cmd.exec(),
        }
        .unwrap_or_else(errors.get_add_err_fn());

        if !errors.is_empty() {
            println!("{}:\n{}", Emoji::new("🚨", "Errors"), style(errors).red());
        }
    }
}
