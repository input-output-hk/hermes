//! CLI interpreter for the service

mod build_info;
mod module;
mod run;

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
}

impl Cli {
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

        logger::init(&log_config).unwrap_or_else(|err| errors.add_err(err));

        match self.command {
            None => run::Run::exec(),
            Some(Commands::Module(cmd)) => cmd.exec(),
        }
        .unwrap_or_else(|err| {
            match err.downcast::<Errors>() {
                Ok(errs) => errors.merge(errs),
                Err(err) => errors.add_err(err),
            }
        });

        if !errors.is_empty() {
            println!("{}:\n{}", Emoji::new("🚨", "Errors"), style(errors).red());
        }
    }
}
