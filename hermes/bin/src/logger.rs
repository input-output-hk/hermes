//! Setup for logging for the service.

use std::str::FromStr;

use clap::ValueEnum;
extern crate derive_more;
use derive_more::Display;
use tracing::{level_filters::LevelFilter, subscriber::SetGlobalDefaultError};
use tracing_subscriber::{
    fmt::{format::FmtSpan, time},
    FmtSubscriber,
};

use crate::runtime_extensions::bindings::hermes::logging::api::Level;

/// All valid logging levels.
#[derive(ValueEnum, Clone, Copy, Display, Default)]
pub(crate) enum LogLevel {
    /// Errors
    #[display(fmt = "Error")]
    Error,
    /// Warnings
    #[display(fmt = "Warn")]
    Warn,
    /// Informational Messages
    #[display(fmt = "Info")]
    #[default]
    Info,
    /// Debug messages
    #[display(fmt = "Debug")]
    Debug,
    /// Tracing
    #[display(fmt = "Trace")]
    Trace,
}

impl FromStr for LogLevel {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Error and Warn levels are force to Info level
        // as Info is the highest log level one can choose.
        match s {
            "error" | "warn" | "info" => Ok(LogLevel::Info),
            "debug" => Ok(LogLevel::Debug),
            "trace" => Ok(LogLevel::Trace),
            _ => Err(()),
        }
    }
}

impl From<Level> for LogLevel {
    fn from(level: Level) -> Self {
        // Error and Warn levels are force to Info level
        // as Info is the highest log level one can choose.
        match level {
            Level::Info | Level::Warn | Level::Error => LogLevel::Info,
            Level::Debug => LogLevel::Debug,
            Level::Trace => LogLevel::Trace,
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
    /// Log level
    log_level: LogLevel,
    /// Enable/disable thread logging
    with_thread: bool,
    /// Enable/disable file logging
    with_file: bool,
    /// Enable/disable line number logging
    with_line_num: bool,
}

#[allow(dead_code)]
impl LoggerConfig {
    /// Build the logger configuration
    pub(crate) fn build(self) -> LoggerConfig {
        self
    }

    /// Set log level
    pub(crate) fn log_level(mut self, level: LogLevel) -> Self {
        self.log_level = level;
        self
    }

    /// Enable/disable thread logging
    pub(crate) fn with_thread(mut self, enable: bool) -> Self {
        self.with_thread = enable;
        self
    }

    /// Enable/disable file logging
    pub(crate) fn with_file(mut self, enable: bool) -> Self {
        self.with_file = enable;
        self
    }

    /// Enable/disable line number logging
    pub(crate) fn with_line_num(mut self, enable: bool) -> Self {
        self.with_line_num = enable;
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
#[allow(dead_code)]
pub(crate) fn init(logger_config: &LoggerConfig) -> Result<(), SetGlobalDefaultError> {
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
        .finish();

    tracing::subscriber::set_global_default(subscriber)
}
