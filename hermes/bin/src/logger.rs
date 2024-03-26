//! Setup for logging for the service.

use clap::ValueEnum;
use tracing::{level_filters::LevelFilter, subscriber::SetGlobalDefaultError};
use tracing_subscriber::{
    fmt::{format::FmtSpan, time},
    FmtSubscriber,
};

/// All valid logging levels.
#[derive(ValueEnum, Clone, Copy)]
pub(crate) enum LogLevel {
    /// Errors
    Error,
    /// Warnings
    Warn,
    /// Informational Messages
    Info,
    /// Debug messages
    Debug,
    /// Tracing
    Trace,
}

impl From<&str> for LogLevel {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "error" => LogLevel::Error,
            "warn" => LogLevel::Warn,
            "debug" => LogLevel::Debug,
            "trace" => LogLevel::Trace,
            _ => LogLevel::Info,
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

/// Initializes the subscriber for the logger with the following features.
/// - JSON format
/// - Display event level
/// - Display thread names and ids
/// - Display event's source code file path and line number
/// - Display time in RFC 3339 format
/// - Events emit when the span close
/// - Maximum verbosity level
pub(crate) fn init(
    log_level: LogLevel, with_thread: bool, with_file: bool, with_line_num: bool,
) -> Result<(), SetGlobalDefaultError> {
    let subscriber = FmtSubscriber::builder()
        .json()
        .with_level(true)
        .with_thread_names(with_thread)
        .with_thread_ids(with_thread)
        .with_file(with_file)
        .with_line_number(with_line_num)
        .with_timer(time::UtcTime::rfc_3339())
        .with_span_events(FmtSpan::CLOSE)
        .with_max_level(LevelFilter::from_level(log_level.into()))
        .finish();

    tracing::subscriber::set_global_default(subscriber)
}
