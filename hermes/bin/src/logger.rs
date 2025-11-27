//! Setup for logging for the service.

use std::str::FromStr;

use derive_more::Display;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::{
    EnvFilter, FmtSubscriber,
    fmt::{format::FmtSpan, time},
};

use crate::runtime_extensions::bindings::hermes::logging;

/// All valid logging levels.
#[derive(Clone, Copy, Display, Default)]
pub(crate) enum LogLevel {
    /// Errors
    #[display("Error")]
    Error,
    /// Warnings
    #[display("Warn")]
    Warn,
    /// Informational Messages
    #[default]
    #[display("Info")]
    Info,
    /// Debug messages
    #[display("Debug")]
    Debug,
    /// Tracing
    #[display("Trace")]
    Trace,
}

impl FromStr for LogLevel {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "error" => Ok(LogLevel::Error),
            "warn" => Ok(LogLevel::Warn),
            "info" => Ok(LogLevel::Info),
            "debug" => Ok(LogLevel::Debug),
            "trace" => Ok(LogLevel::Trace),
            _ => Err(anyhow::anyhow!("Invalid log level string: {s}")),
        }
    }
}

impl From<logging::api::Level> for LogLevel {
    fn from(level: logging::api::Level) -> Self {
        // Error and Warn levels are force to Info level
        // as Info is the highest log level one can choose.
        match level {
            logging::api::Level::Info => LogLevel::Info,
            logging::api::Level::Warn => LogLevel::Warn,
            logging::api::Level::Error => LogLevel::Error,
            logging::api::Level::Debug => LogLevel::Debug,
            logging::api::Level::Trace => LogLevel::Trace,
        }
    }
}

/// Implements a conversion from `LogLevel` enum to the `tracing::Level`.
impl From<LogLevel> for tracing::Level {
    fn from(val: LogLevel) -> Self {
        match val {
            LogLevel::Error => Self::ERROR,
            LogLevel::Warn => Self::WARN,
            LogLevel::Info => Self::INFO,
            LogLevel::Debug => Self::DEBUG,
            LogLevel::Trace => Self::TRACE,
        }
    }
}

/// Logger configuration.
#[derive(Default)]
pub(crate) struct LoggerConfig {
    /// Log level.
    log_level: LogLevel,
    /// Enable/disable thread logging.
    with_thread: bool,
    /// Enable/disable file logging.
    with_file: bool,
    /// Enable/disable line number logging.
    with_line_num: bool,
}

/// Logger configuration builder.
#[derive(Default, Clone)]
pub(crate) struct LoggerConfigBuilder {
    /// Builder log level.
    log_level: Option<LogLevel>,
    /// Builder enable/disable thread logging.
    with_thread: Option<bool>,
    /// Builder enable/disable file logging.
    with_file: Option<bool>,
    /// Builder enable/disable line number logging.
    with_line_num: Option<bool>,
}

impl LoggerConfigBuilder {
    /// Build the logger configuration.
    pub(crate) fn build(self) -> LoggerConfig {
        LoggerConfig {
            log_level: self.log_level.unwrap_or_default(),
            with_thread: self.with_thread.unwrap_or(false),
            with_file: self.with_file.unwrap_or(false),
            with_line_num: self.with_line_num.unwrap_or(false),
        }
    }

    /// Set log level.
    pub(crate) fn log_level(
        mut self,
        level: LogLevel,
    ) -> Self {
        self.log_level = Some(level);
        self
    }

    /// Enable/disable thread logging.
    pub(crate) fn with_thread(
        mut self,
        enable: bool,
    ) -> Self {
        self.with_thread = Some(enable);
        self
    }

    /// Enable/disable file logging.
    pub(crate) fn with_file(
        mut self,
        enable: bool,
    ) -> Self {
        self.with_file = Some(enable);
        self
    }

    /// Enable/disable line number logging.
    pub(crate) fn with_line_num(
        mut self,
        enable: bool,
    ) -> Self {
        self.with_line_num = Some(enable);
        self
    }
}

/// Initializes the subscriber for the logger with the following features.
/// - JSON format
/// - Display event level
/// - Display thread names and ids
/// - Display event's source code file path and line number
/// - Display time in RFC 3339 format
/// - Events emit when the span close
/// - Maximum verbosity level
pub(crate) fn init(logger_config: &LoggerConfig) -> anyhow::Result<()> {
    let subscriber = FmtSubscriber::builder()
        .json()
        .with_level(true)
        .with_thread_names(logger_config.with_thread)
        .with_thread_ids(logger_config.with_thread)
        .with_file(logger_config.with_file)
        .with_line_number(logger_config.with_line_num)
        .with_timer(time::UtcTime::rfc_3339())
        .with_span_events(FmtSpan::CLOSE)
        .with_max_level(LevelFilter::from_level(logger_config.log_level.into()))
        // Hardcode the filter to always suppress excess noise
        .with_env_filter(EnvFilter::new("hermes=info,rust_ipfs=error"))
        .finish();

    Ok(tracing::subscriber::set_global_default(subscriber)?)
}
