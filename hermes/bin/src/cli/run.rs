//! Run cli command

use std::str::FromStr;

use crate::{
    logger::{self, LogLevel, LoggerConfigBuilder},
    reactor::HermesReactor,
};

/// A parameter identifier specifying the log level.
const ENV_LOG_LEVEL: &str = "HERMES_LOG_LEVEL";

/// Run cli command
pub(crate) struct Run;

impl Run {
    /// Run the hermes
    pub(crate) fn exec() -> anyhow::Result<()> {
        let log_level = if let Ok(log_level_str) = std::env::var(ENV_LOG_LEVEL) {
            LogLevel::from_str(&log_level_str)?
        } else {
            LogLevel::default()
        };

        let log_config = LoggerConfigBuilder::default()
            .log_level(log_level)
            .with_thread(true)
            .with_file(true)
            .with_line_num(true)
            .build();
        logger::init(&log_config)?;

        let mut reactor = HermesReactor::new(vec![])?;
        reactor.wait()?;

        Ok(())
    }
}
