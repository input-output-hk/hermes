//! Implementation of logging API
use tracing::info;

/// Log a message
pub(crate) fn log_message(
    ctx: Option<String>, msg: &str, file: Option<String>, function: Option<String>,
    line: Option<u32>, col: Option<u32>, data: Option<String>,
) {
    // Force the log level to be info
    info!(
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
    use crate::logger::{init, LogLevel};

    #[test]
    fn test_log_message() {
        if let Err(err) = init(LogLevel::Info, false, false, false) {
            println!("Error initializing logger: {err}");
        }

        // Test with valid data
        let ctx = Some("Context".to_string());
        let msg = "Test message";
        let file = Some("test.rs".to_string());
        let function = Some("test_log_message".to_string());
        let line = Some(10);
        let col = Some(5);
        let data = Some("{\"bt\": [\"Array:1\", \"Array:2\", \"Array:3\"]}".to_string());

        log_message(
            ctx.clone(),
            msg,
            file.clone(),
            function.clone(),
            line,
            col,
            data.clone(),
        );

        log_message(ctx, msg, file, function, line, col, None);
    }
}
