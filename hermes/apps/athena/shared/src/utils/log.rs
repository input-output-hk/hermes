//! Logging utilities.

use std::sync::Once;

use log::Log;
pub use log::{debug, error, info, log_enabled, trace, warn, LevelFilter};

/// Compatibility between [`hermes::logging`] and [`log`].
mod compat {
    use crate::bindings::hermes;

    /// Converts from [`log::Level`] to the compatible level from bindings.
    fn convert_level(compat: log::Level) -> hermes::logging::api::Level {
        match compat {
            log::Level::Error => hermes::logging::api::Level::Error,
            log::Level::Warn => hermes::logging::api::Level::Warn,
            log::Level::Info => hermes::logging::api::Level::Info,
            log::Level::Debug => hermes::logging::api::Level::Debug,
            log::Level::Trace => hermes::logging::api::Level::Trace,
        }
    }

    /// Serializes logged key-value pairs to json.
    #[derive(Default)]
    struct DataVisitor(serde_json::Map<String, serde_json::Value>);

    impl<'kvs> log::kv::VisitSource<'kvs> for DataVisitor {
        fn visit_pair(
            &mut self,
            key: log::kv::Key<'kvs>,
            value: log::kv::Value<'kvs>,
        ) -> Result<(), log::kv::Error> {
            let value = serde_json::to_value(&value)
                .unwrap_or_else(|_| serde_json::Value::String(value.to_string()));
            self.0.insert(key.as_str().to_owned(), value);
            Ok(())
        }
    }

    /// Hermes compatible log record.
    pub struct Record<'a> {
        level: hermes::logging::api::Level,
        file: Option<&'a str>,
        line: Option<u32>,
        function: Option<String>,
        ctx: Option<&'a str>,
        msg: String,
        data: Option<String>,
    }

    impl<'a> From<&'a log::Record<'a>> for Record<'a> {
        fn from(value: &'a log::Record<'a>) -> Self {
            Self {
                level: convert_level(value.level()),
                file: value.file(),
                line: value.line(),
                // Function cannot be easily retrieved, so it is anonymized as "xxx".
                function: value
                    .module_path()
                    .map(|module_path| format!("{module_path}::xxx")),
                // Target maps to context.
                ctx: Some(value.target()),
                // Arguments are formatted same way `print!` formats them.
                msg: format!("{}", value.args()),
                // Structured data becomes json.
                data: {
                    let mut visitor = DataVisitor::default();
                    // Data serialization should not return errors by implementation.
                    let _ = value.key_values().visit(&mut visitor);
                    (!visitor.0.is_empty())
                        .then(|| serde_json::to_string(&visitor.0).ok())
                        .flatten()
                },
            }
        }
    }

    impl Record<'_> {
        /// Log the record through Hermes logging api.
        pub fn log(self) {
            hermes::logging::api::log(
                self.level,
                self.file,
                self.function.as_deref(),
                self.line,
                None,
                self.ctx,
                &self.msg,
                self.data.as_deref(),
            );
        }
    }
}

/// Forwards [`log`] macros input to Hermes WASM bindings.
struct LogImpl;

impl Log for LogImpl {
    fn enabled(
        &self,
        metadata: &log::Metadata,
    ) -> bool {
        metadata.level() <= log::max_level()
    }

    fn log(
        &self,
        record: &log::Record,
    ) {
        if self.enabled(record.metadata()) {
            compat::Record::from(record).log();
        }
    }

    fn flush(&self) {}
}

/// Initialize [`log`] macros. This must be called at least once to enable logging.
pub fn init(filter: LevelFilter) {
    const ONCE: Once = Once::new();
    ONCE.call_once(move || {
        let _ = log::set_logger(&LogImpl);
        log::set_max_level(filter);
    });
}

// TODO: replace `log_error` calls with `log::info!` macro invocations.
/// Error logging.
pub fn log_error(
    _: &str,
    _: &str,
    context: &str,
    msg: &str,
    data: Option<&str>,
) {
    log::error!(target: context, data; "{msg}");
}

// TODO: replace `log_info` calls with `log::info!` macro invocations.
/// Info logging.
pub fn log_info(
    _: &str,
    _: &str,
    context: &str,
    msg: &str,
    data: Option<&str>,
) {
    log::info!(target: context, data; "{msg}");
}

// TODO: replace `log_warn` calls with `log::warn!` macro invocations.
/// Info logging.
pub fn log_warn(
    _: &str,
    _: &str,
    context: &str,
    msg: &str,
    data: Option<&str>,
) {
    log::warn!(target: context, data; "{msg}");
}
