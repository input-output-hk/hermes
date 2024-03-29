//! Logging host implementation for WASM runtime.

use super::log_msg::log_message;
use crate::{
    logger::LogLevel,
    runtime_context::HermesRuntimeContext,
    runtime_extensions::bindings::hermes::{
        json::api::Json,
        logging::api::{Host, Level},
    },
};

impl Host for HermesRuntimeContext {
    /// Generate a Log
    ///
    /// The Hermes API will add extra information to the log, such as the instance of the
    /// webasm
    /// module being logged.
    /// The Webasm module does not need to concern itself with this kind of information,
    /// and should
    /// log as if it is the only instance.
    /// It also should not log any webasm shared context, except where it is relevant to
    /// the log message itself.
    ///
    /// **Parameters**
    ///
    /// - `level` : The log level this message is for.
    /// - `file`  : The name of the src file being logged from. (Optional)
    /// - `function`    : The function within the file being logged from. (Optional)
    /// - `line`  : The line of code the log was generated from. (Optional)
    /// - `col`   : The column of code the log was generated from. (Optional)
    /// - `ctx`   : The logging context.  (Should have no newlines or formatting).
    /// - `msg`   : A Single line message to be logged. (Should have no newlines or
    ///   formatting).
    /// - `data`  : A Free form json payload that will be logged with the msg.  This must
    ///   be valid JSON.
    ///
    /// *Notes*
    ///
    /// The `data` parameter may contain a record of the format:
    /// ```json
    /// {
    /// "bt" : [ <string> , <string> ]
    /// }
    /// ```
    /// The logger will interpret this as a backtrace where each entry in the array is one
    /// line of the backtrace.
    /// The format of the backtrace lines is up to the webasm module generating the log.
    /// The individual backtrace entries may contain line breaks if the backtrace entry is
    /// multiline.
    /// * Multiline backtrace entries should be de-dented, relative to the first line.
    /// * This is to allow the display to properly format multiline entries.
    /// This format is designed to keep the broadest flexibility for multiple languages
    /// capabilities.
    /// The backtrace must be sorted with most recent lines of the backtrace occurring
    /// first in the array.
    /// Backtrace must be contained in a single `log` call.  Multiple log calls will be
    /// considered independent logs.
    fn log(
        &mut self, level: Level, file: Option<String>, function: Option<String>, line: Option<u32>,
        col: Option<u32>, ctx: Option<String>, msg: String, data: Option<Json>,
    ) -> wasmtime::Result<()> {
        log_message(
            LogLevel::from(level),
            ctx,
            &msg,
            file,
            function,
            line,
            col,
            data,
        );
        Ok(())
    }
}
