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

    let log_level = match LogLevel::from_str(level) {
        LogLevel::Error => Level::ERROR,
        LogLevel::Warn => Level::WARN,
        LogLevel::Info => Level::INFO,
        LogLevel::Debug => Level::DEBUG,
        LogLevel::Trace => Level::TRACE,
    };

    if let Value::Object(obj) = parsed_data {
        for (_key, value) in obj {
            // FIXME - Fix level and span name
            let span = span!(Level::INFO, "log_span");
            span.in_scope(|| {
                if let Some(backtrace) = value.as_array() {
                    for entry in backtrace {
                        if let Some(entry) = entry.as_str() {
                            match log_level {
                                Level::TRACE => trace!(
                                    ctx = ctx,
                                    message = msg,
                                    file = file,
                                    function = function,
                                    line = line,
                                    column = col
                                ),
                                Level::DEBUG => debug!(
                                    ctx = ctx,
                                    message = msg,
                                    file = file,
                                    function = function,
                                    line = line,
                                    column = col
                                ),
                                Level::INFO => info!(
                                    ctx = ctx,
                                    message = msg,
                                    file = file,
                                    function = function,
                                    line = line,
                                    column = col,
                                    entry = entry
                                ),
                                Level::WARN => warn!(
                                    ctx = ctx,
                                    message = msg,
                                    file = file,
                                    function = function,
                                    line = line,
                                    column = col
                                ),
                                Level::ERROR => error!(
                                    ctx = ctx,
                                    message = msg,
                                    file = file,
                                    function = function,
                                    line = line,
                                    column = col
                                ),
                            }
                        }
                    }
                }
            });
        }
    } else {
        println!("Invalid backtrace format: {:?}", parsed_data);
    }
}

#[cfg(test)]
mod tests_log_msg {

    use crate::logger;

    use super::*;
    #[test]
    fn test_log_message() {
        if let Err(err) = logger::init(LogLevel::from_str("info"), true, true, true) {
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
