use tokio::sync::{mpsc, oneshot};
use tracing::{error, trace};
use thiserror::Error;

use crate::{event::{HermesEvent, HermesEventPayload, TargetApp, TargetModule}, runtime_extensions::bindings::hermes::http_request::api::Payload};

pub enum Command {
    Send { payload: Payload, send_result_sender: oneshot::Sender<bool> },
}

type CommandSender = tokio::sync::mpsc::Sender<Command>;

type CommandReceiver = tokio::sync::mpsc::Receiver<Command>;

pub struct TokioTaskHandle {
    cmd_tx: CommandSender,
}

#[derive(Error, Debug)]
pub enum RequestSendingError {
    #[error("Failed to send command via channel: {0}")]
    ChannelSend(#[from] mpsc::error::SendError<Command>),
    #[error("Failed to receive command result via channel: {0}")]
    ResponseReceive(#[from] oneshot::error::RecvError),
}

impl TokioTaskHandle {
    /// Sends a command to the Tokio runtime task.
    pub fn send(
        &self, payload: Payload,
    ) -> Result<bool, RequestSendingError> {
        let (send_result_sender, send_result_receiver) = tokio::sync::oneshot::channel();
        self.cmd_tx.blocking_send(Command::Send { payload, send_result_sender })?;
        let sending_result = send_result_receiver.blocking_recv()?;
        error!(%sending_result, "Got sending result");
        Ok(sending_result)
    }
}

pub fn spawn() -> TokioTaskHandle {
    let (cmd_tx, cmd_rx) = tokio::sync::mpsc::channel(1);
    std::thread::spawn(move || {
        executor(cmd_rx);
    });

    TokioTaskHandle { cmd_tx }
}

pub(crate) struct ParsedPayload {
    pub(crate) body_str: String,
    pub(crate) request_line: String,
    pub(crate) url: String,
    pub(crate) request_id: String,
}

pub(crate) fn parse_payload(payload: Payload) -> ParsedPayload {
    let body_str = String::from_utf8(payload.body).unwrap();
    let request_line = body_str
        .lines()
        .next()
        .ok_or("Empty HTTP body")
        .unwrap()
        .to_string();

    let parts: Vec<&str> = request_line.split_whitespace().collect();
    if parts.len() < 2 {
        tracing::error!("E1");
    }
    let path = parts[1];

    let scheme = if payload.host_uri.starts_with("https") {
        "https"
    } else {
        "http"
    };
    let domain = payload
        .host_uri
        .trim_start_matches("http://")
        .trim_start_matches("https://");

    let url = format!("{}://{}:{}{}", scheme, domain, payload.port, path);
    tracing::error!("Full URL: {}", url);
    ParsedPayload {
        body_str,
        request_line,
        url,
        request_id: payload.request_id.clone().unwrap(),
    }
}


fn send_request_in_background(request_id: String, body_str: String, request_line: String, url: String) -> bool {
    std::thread::spawn(move || {
        let client = reqwest::blocking::Client::new(); // TODO: Reuse client
        let response = if request_line.starts_with("POST") {
            let body_content = body_str.split("\r\n\r\n").last().unwrap_or("");
            client
                .post(&url)
                .body(body_content.to_string())
                .send()
                .unwrap()
        } else {
            client.get(&url).send().unwrap()
        };
    
        let response_text = response
            .text()
            .unwrap_or_else(|_| "Failed to read response".to_string());

        let on_http_response_event = super::event::OnHttpResponseEvent {
            request_id,
            response: response_text,
        };
    
        crate::event::queue::send(HermesEvent::new(
            on_http_response_event,
            TargetApp::All,
            TargetModule::All,
        ));
    }
    );

    true
}

fn executor(mut cmd_rx: CommandReceiver) {
    let res = tokio::runtime::Builder::new_current_thread()
        .enable_io()
        .enable_time()
        .build();

    let rt = match res {
        Ok(rt) => rt,
        Err(err) => {
            error!(error = ?err, "Failed to start Http Request Runtime Extension background thread");
            return;
        },
    };

    trace!("Created Tokio runtime for Http Request Runtime Extension");

    rt.block_on(async move {
        while let Some(cmd) = cmd_rx.recv().await {
            match cmd {
                Command::Send { payload, send_result_sender } => {
                    let ParsedPayload {
                        body_str,
                        request_line,
                        url,
                        request_id,
                    } = parse_payload(payload);
                    error!(body_str = %body_str, request_line = %request_line, url = %url, request_id = %request_id, "Parsed payload");
                
                    let sending_result = send_request_in_background(request_id, body_str,
                        request_line,
                        url);
                    let _ = send_result_sender.send(sending_result);
                },
            }
        }
    });
}
