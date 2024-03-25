use crate::logger::LogLevel;
use serde_json::{from_str, Value};
use tracing::{debug, error, info, span, trace, warn, Level};

#[allow(dead_code)]
pub(crate) fn log_message(
    level: &str, ctx: Option<&str>, msg: &str, file: Option<&str>, function: Option<&str>,
    line: Option<u32>, col: Option<u32>, data: Option<&str>,
) {
    // Parse the JSON data if provided
    let parsed_data: Value = data
        .and_then(|data| from_str(&data).ok())
        .unwrap_or_default();

    if let Value::Object(obj) = parsed_data {
        for (key, value) in obj {
            // FIXME - Fix level and span name
            let span = span!(Level::INFO, "log_span", %key);
            span.in_scope(|| {
                if let Some(data) = value.as_array() {
                    for entry in data {
                        if let Some(entry) = entry.as_str() {
                            log_with_context(
                                LogLevel::from(level),
                                ctx,
                                msg,
                                file,
                                function,
                                line,
                                col,
                                entry,
                            )
                        }
                    }
                }
            });
        }
    }
}

/// Log the message with the information and its level.
fn log_with_context(
    level: LogLevel, ctx: Option<&str>, msg: &str, file: Option<&str>, function: Option<&str>,
    line: Option<u32>, col: Option<u32>, entry: &str,
) {
    match level {
        LogLevel::Trace => trace!(
            ctx = ctx,
            message = msg,
            file = file,
            function = function,
            line = line,
            column = col,
            entry = entry
        ),
        LogLevel::Debug => debug!(
            ctx = ctx,
            message = msg,
            file = file,
            function = function,
            line = line,
            column = col,
            entry = entry
        ),
        LogLevel::Info => info!(
            ctx = ctx,
            message = msg,
            file = file,
            function = function,
            line = line,
            column = col,
            entry = entry
        ),
        LogLevel::Warn => warn!(
            ctx = ctx,
            message = msg,
            file = file,
            function = function,
            line = line,
            column = col,
            entry = entry
        ),
        LogLevel::Error => error!(
            ctx = ctx,
            message = msg,
            file = file,
            function = function,
            line = line,
            column = col,
            entry = entry
        ),
    }
}

#[cfg(test)]
mod tests_log_msg {

    use crate::logger;

    use super::*;
    #[test]
    fn test_log_message() {
        if let Err(err) = logger::init(LogLevel::Info, true, true, true) {
            println!("Error initializing logger: {err}");
        }

        // Test with valid data
        let level = "info";
        let ctx = Some("Context");
        let msg = "Test message";
        let file = Some("test.rs");
        let function = Some("test_log_message");
        let line = Some(10);
        let col = Some(5);
        let data = Some("{\"bt\": [\"Array:1\", \"Array:2\", \"Array:3\"]}");

        log_message(level, ctx, msg, file, function, line, col, data);
    }
}
