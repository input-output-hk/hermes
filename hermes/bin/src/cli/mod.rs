//! CLI interpreter for the service

mod app;
mod build_info;
mod module;
mod playground;
mod run;

use std::{path::PathBuf, process::ExitCode};

use build_info::BUILD_INFO;
use clap::{Args, Parser, Subcommand};
use console::{Emoji, style};
use tracing::error;

use crate::{
    errors::Errors,
    logger::{self, LoggerConfigBuilder},
    runtime_extensions::init::trait_runtime::{RteInitRuntime, RteRuntime},
};

/// A parameter identifier specifying the log level.
#[allow(dead_code)]
const ENV_LOG_LEVEL: &str = "HERMES_LOG_LEVEL";

/// An exit code returned on non-application errors.
/// This is consistent with `cargo test` return code.
#[allow(dead_code)]
const INTERNAL_FAILURE_CODE: u8 = 101;

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
    command: Commands,
}

/// Hermes cli commands
#[derive(Subcommand)]
enum Commands {
    /// Run the hermes node
    Run(run::Run),
    /// module commands
    #[clap(subcommand)]
    Module(module::Commands),
    /// app commands
    #[clap(subcommand)]
    App(app::Commands),
    /// Run the hermes playground
    Playground(playground::Playground),
}

impl Cli {
    /// Hermes home directory
    #[allow(dead_code)]
    pub(crate) fn hermes_home() -> anyhow::Result<PathBuf> {
        let hermes_home = dirs::home_dir()
            .ok_or(anyhow::anyhow!(
                "Current platform does not have a home directory"
            ))?
            .join(".hermes");
        std::fs::create_dir_all(&hermes_home)?;
        Ok(hermes_home)
    }

    /// Execute cli commands of the hermes
    #[allow(dead_code)]
    pub(crate) fn exec(self) -> ExitCode {
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

        // Initialize all runtime extensions before doing anything with apps or modules.
        // TODO (SJ): Better handle errors.
        if let Err(err) = RteRuntime::new().init() {
            error!(err=%err,"Runtime Extension Node Init Failed");
            return ExitCode::FAILURE;
        }

        let mut exit_code = match self.command {
            Commands::Run(cmd) => cmd.exec(),
            Commands::Module(cmd) => cmd.exec().map(|()| ExitCode::SUCCESS.into()),
            Commands::App(cmd) => cmd.exec().map(|()| ExitCode::SUCCESS.into()),
            Commands::Playground(playground) => playground.exec(),
        }
        .and_then(|exit| {
            exit.get_exit_code()
                .ok_or_else(|| anyhow::Error::from(exit))
        })
        .map_err(errors.get_add_err_fn())
        .unwrap_or(ExitCode::from(INTERNAL_FAILURE_CODE));

        if !errors.is_empty() {
            error!("{}:\n{}", Emoji::new("🚨", "Errors"), style(errors).red());
            exit_code = ExitCode::FAILURE;
            // Keep going so we can finalize the runtime cleanly.
        }

        // Cleanup all runtime extensions before exiting (after all apps and modules are done).
        // TODO (SJ): Cleanup error reporting.
        if let Err(err) = RteRuntime::new().fini() {
            error!(err=%err,"Runtime Extension Node Finalization Failed");
            exit_code = ExitCode::FAILURE;
            // Keep going exit_code reported below anyway.
        }

        exit_code
    }
}

/// Additional Hermes run arguments
#[derive(Args, Debug)]
pub(crate) struct RuntimeConfig {
    /// Shutdown Hermes after the timeout (milliseconds)
    #[arg(long)]
    timeout_ms: Option<u64>,

    /// Disables parallel execution of event handlers
    #[arg(long, default_value_t = false)]
    no_parallel: bool,

    /// Serializes `SQLite` database access
    #[arg(long, default_value_t = false)]
    serialize_sqlite: bool,
}
