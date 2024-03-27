//! Implementation of logging API
use tracing::info;

use crate::logger::LogLevel;

/// Log a message
#[allow(clippy::too_many_arguments)]
pub(crate) fn log_message(
    level: LogLevel, ctx: Option<String>, msg: &str, file: Option<String>,
    function: Option<String>, line: Option<u32>, col: Option<u32>, data: Option<String>,
) {
    info!(
        level = level.to_string(),
        ctx = ctx.unwrap_or_default(),
        message = msg,
        file = file.unwrap_or_default(),
        function = function.unwrap_or_default(),
        line = line.unwrap_or_default(),
        column = col.unwrap_or_default(),
        data = data.unwrap_or_default(),
    );
}

#[cfg(test)]
mod tests_log_msg {
    use super::*;
    use crate::{
        logger::{init, LogLevel, LoggerConfig},
        runtime_extensions::bindings::hermes::logging::api::Level,
    };

    #[test]
    fn test_log_message() {
        let config = LoggerConfig::default();

        if let Err(err) = init(&config) {
            println!("Error initializing logger: {err}");
        }

        // Test with valid data
        let level = Level::Warn;
        let ctx = Some("Context".to_string());
        let msg = "Test message";
        let file = Some("test.rs".to_string());
        let function = Some("test_log_message".to_string());
        let line = Some(10);
        let col = Some(5);
        let data = Some("{\"bt\": [\"Array:1\", \"Array:2\", \"Array:3\"]}".to_string());

        log_message(
            LogLevel::from(level),
            ctx.clone(),
            msg,
            file.clone(),
            function.clone(),
            line,
            col,
            data.clone(),
        );

        log_message(
            LogLevel::from(level),
            ctx,
            msg,
            file,
            function,
            line,
            col,
            None,
        );
    }
}
