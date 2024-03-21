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
}

/// Implements a conversion from LogLevel enum to the tracing::Level.
impl From<LogLevel> for tracing::Level {
    fn from(val: LogLevel) -> Self {
        match val {
            LogLevel::Error => Self::ERROR,
            LogLevel::Warn => Self::WARN,
            LogLevel::Info => Self::INFO,
            LogLevel::Debug => Self::DEBUG,
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
fn init_subscriber(log_level: LogLevel) -> Result<(), SetGlobalDefaultError> {
    let subscriber = FmtSubscriber::builder()
        .json()
        .with_level(true)
        .with_thread_names(true)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true)
        .with_timer(time::UtcTime::rfc_3339())
        .with_span_events(FmtSpan::CLOSE)
        .with_max_level(LevelFilter::from_level(log_level.into()))
        .finish();

    tracing::subscriber::set_global_default(subscriber)
}

/// Initializes the subscriber with the given log level.
pub(crate) fn init(log_level: LogLevel) -> Result<(), SetGlobalDefaultError> {
    init_subscriber(log_level)
}
