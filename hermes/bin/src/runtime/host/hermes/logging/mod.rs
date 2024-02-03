//! Host - Logging implementations
//!
#![allow(unused_variables)]

use crate::runtime::extensions::{
    hermes::{
        json::api::Json,
        logging::api::{Host, Level},
    },
    HermesState, NewState,
};

/// State
pub(crate) struct State {}

impl NewState for State {
    fn new(_ctx: &crate::wasm::context::Context) -> Self {
        State {}
    }
}

impl Host for HermesState {
    #[doc = " Generate a Log"]
    #[doc = " "]
    #[doc = " The Hermes API will add extra information to the log, such as the instance of the webasm"]
    #[doc = " module being logged."]
    #[doc = " The Webasm module does not need to concern itself with this kind of information, and should"]
    #[doc = " log as if it is the only instance."]
    #[doc = " It also should not log any webasm shared context, except where it is relevant to the log message itself."]
    #[doc = " "]
    #[doc = " **Parameters**"]
    #[doc = " "]
    #[doc = " - `level` : The log level this message is for."]
    #[doc = " - `file`  : The name of the src file being logged from. (Optional)"]
    #[doc = " - `fn`    : The function within the file being logged from. (Optional)"]
    #[doc = " - `line`  : The line of code the log was generated from. (Optional)"]
    #[doc = " - `col`   : The column of code the log was generated from. (Optional)"]
    #[doc = " - `ctx`   : The logging context.  (Should have no newlines or formatting)."]
    #[doc = " - `msg`   : A Single line message to be logged. (Should have no newlines or formatting)."]
    #[doc = " - `data`  : A Free form json payload that will be logged with the msg.  This must be valid JSON."]
    #[doc = " "]
    #[doc = " *Notes*"]
    #[doc = " "]
    #[doc = " The `data` parameter may contain a record of the format:"]
    #[doc = " ```json"]
    #[doc = " {"]
    #[doc = " \"bt\" : [ <string> , <string> ]"]
    #[doc = " }"]
    #[doc = " ```"]
    #[doc = " The logger will interpret this as a backtrace where each entry in the array is one line of the backtrace."]
    #[doc = " The format of the backtrace lines is up to the webasm module generating the log."]
    #[doc = " The individual backtrace entries may contain line breaks if the backtrace entry is"]
    #[doc = " multiline."]
    #[doc = " * Multiline backtrace entries should be de-dented, relative to the first line."]
    #[doc = " * This is to allow the display to properly format multiline entries."]
    #[doc = " This format is designed to keep the broadest flexibility for multiple languages capabilities."]
    #[doc = " The backtrace must be sorted with most recent lines of the backtrace occurring first in the array."]
    #[doc = " Backtrace must be contained in a single `log` call.  Multiple log calls will be considered independent logs."]
    fn log(
        &mut self, level: Level, file: Option<String>, fn_: Option<String>, line: Option<u32>,
        col: Option<u32>, ctx: Option<String>, msg: String, data: Option<Json>,
    ) -> wasmtime::Result<()> {
        todo!()
    }
}
