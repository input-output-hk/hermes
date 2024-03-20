//! Setup for logging for the service.

use clap::ValueEnum;
use tracing::{level_filters::LevelFilter, subscriber::SetGlobalDefaultError};
use tracing_subscriber::{
    fmt::{format::FmtSpan, time},
    FmtSubscriber,
};

#[derive(ValueEnum, Clone)]

/// Log formats
pub enum LogFormat {
    /// JSON format
    Json,
    /// Pretty format
    Pretty,
    /// Compact format
    Compact,
    /// Full format
    Full,
}

/// All valid logging levels
#[derive(ValueEnum, Clone, Copy)]
pub(crate) enum LogLevel {
    /// Debug messages
    Debug,
    /// Informational Messages
    Info,
    /// Warnings
    Warn,
    /// Errors
    Error,
}

impl From<LogLevel> for tracing::Level {
    fn from(val: LogLevel) -> Self {
        match val {
            LogLevel::Debug => Self::DEBUG,
            LogLevel::Info => Self::INFO,
            LogLevel::Warn => Self::WARN,
            LogLevel::Error => Self::ERROR,
        }
    }
}

fn init_subscriber(format: LogFormat, log_level: LogLevel) -> Result<(), SetGlobalDefaultError> {
    let subscriber = FmtSubscriber::builder()
        .with_level(true)
        .with_thread_names(true)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true)
        .with_timer(time::UtcTime::rfc_3339())
        .with_span_events(FmtSpan::CLOSE)
        .with_max_level(LevelFilter::from_level(log_level.into()));

    match format {
        LogFormat::Json => tracing::subscriber::set_global_default(subscriber.json().finish()),
        LogFormat::Pretty => {
            tracing::subscriber::set_global_default(subscriber.with_ansi(true).pretty().finish())
        },
        LogFormat::Compact => {
            tracing::subscriber::set_global_default(subscriber.compact().finish())
        },
        LogFormat::Full => tracing::subscriber::set_global_default(subscriber.finish()),
    }
}

pub fn init(log_format: LogFormat, log_level: LogLevel) -> Result<(), SetGlobalDefaultError> {
    init_subscriber(log_format, log_level)
}
