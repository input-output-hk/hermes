#[allow(clippy::all, unused)]
mod hermes;
mod stub;

use crate::hermes::exports::hermes::http_gateway::event::HttpGatewayResponse;
use crate::hermes::exports::hermes::http_gateway::event::HttpResponse;
use crate::hermes::hermes::binary::api::Bstr;

struct VotingIndexComponent;

/// Logs an info message
fn log_info(message: &str) {
    hermes::hermes::logging::api::log(
        hermes::hermes::logging::api::Level::Info,
        Some("voting-index"),
        None,
        None,
        None,
        None,
        message,
        None,
    );
}

/// Logs a debug message
fn log_debug(message: &str) {
    hermes::hermes::logging::api::log(
        hermes::hermes::logging::api::Level::Debug,
        Some("voting-index"),
        None,
        None,
        None,
        None,
        message,
        None,
    );
}

/// Logs a warning message
fn log_warn(message: &str) {
    hermes::hermes::logging::api::log(
        hermes::hermes::logging::api::Level::Warn,
        Some("voting-index"),
        None,
        None,
        None,
        None,
        message,
        None,
    );
}

impl hermes::exports::hermes::http_gateway::event::Guest for VotingIndexComponent {
    fn reply(
        _body: Vec<u8>,
        _headers: hermes::exports::hermes::http_gateway::event::Headers,
        path: String,
        method: String,
    ) -> Option<HttpGatewayResponse> {
        log_info(&format!("Processing HTTP request: {} {}", method, path));

        Some(HttpGatewayResponse::Http(HttpResponse {
            code: 200,
            headers: vec![("content-type".to_string(), vec!["text/plain".to_string()])],
            body: Bstr::from(format!("Voting index stuff: {}", path)),
        }))
    }
}

hermes::export!(VotingIndexComponent with_types_in hermes);
