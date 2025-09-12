//! Logging utilities.

// TODO - This should be removed once <https://github.com/input-output-hk/hermes/issues/505> is implemented.
use crate::hermes::logging::api::{log, Level};

/// Error logging.
pub(crate) fn log_error(
    file: &str,
    function: &str,
    context: &str,
    msg: &str,
    data: Option<&str>,
) {
    log(
        Level::Error,
        Some(file),
        Some(function),
        None,
        None,
        Some(context),
        &format!("🚨 {msg}"),
        data,
    );
}

/// Info logging.
pub(crate) fn log_info(
    file: &str,
    function: &str,
    context: &str,
    msg: &str,
    data: Option<&str>,
) {
    log(
        Level::Info,
        Some(file),
        Some(function),
        None,
        None,
        Some(context),
        msg,
        data,
    );
}
