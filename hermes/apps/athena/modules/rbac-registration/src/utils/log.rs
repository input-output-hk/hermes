use crate::hermes;

pub(crate) fn log_error(
    file: &str,
    function: &str,
    context: &str,
    msg: &str,
    data: Option<&str>,
) {
    hermes::hermes::logging::api::log(
        hermes::hermes::logging::api::Level::Error,
        Some(file),
        Some(function),
        None,
        None,
        Some(context),
        msg,
        data,
    );
}

pub(crate) fn log_info(
    file: &str,
    function: &str,
    context: &str,
    msg: &str,
    data: Option<&str>,
) {
    hermes::hermes::logging::api::log(
        hermes::hermes::logging::api::Level::Info,
        Some(file),
        Some(function),
        None,
        None,
        Some(context),
        msg,
        data,
    );
}

