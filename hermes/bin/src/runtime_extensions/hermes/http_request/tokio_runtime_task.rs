use std::sync::OnceLock;
use tracing::{error, trace};

use crate::runtime_extensions::hermes::http_request::Payload;
use crate::event::{HermesEvent, TargetApp, TargetModule};


enum Command {
    Send { payload: Payload },
}

type CommandSender = tokio::sync::mpsc::Sender<Command>;
type CommandReceiver = tokio::sync::mpsc::Receiver<Command>;

// Custom error type for HTTP requests
#[derive(Debug)]
pub enum HttpError {
    InvalidUtf8,
    EmptyBody,
    InvalidRequestLine,
    NetworkError(reqwest::Error),
}

impl std::fmt::Display for HttpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HttpError::InvalidUtf8 => write!(f, "Invalid UTF-8 in body"),
            HttpError::EmptyBody => write!(f, "Empty HTTP body"),
            HttpError::InvalidRequestLine => write!(f, "Invalid HTTP request line"),
            HttpError::NetworkError(e) => write!(f, "Network error: {}", e),
        }
    }
}

impl std::error::Error for HttpError {}

impl From<reqwest::Error> for HttpError {
    fn from(err: reqwest::Error) -> Self {
        HttpError::NetworkError(err)
    }
}

pub struct Handle {
    cmd_tx: CommandSender,
}

impl Handle {
    pub(super) fn send(&self, payload: Payload) -> Result<bool, super::Error> {
        self.cmd_tx
            .blocking_send(Command::Send { payload })
            .map_err(|_| 0u32)?;

        Ok(true)
    }
}

// Global handle storage
static HTTP_HANDLE: OnceLock<Handle> = OnceLock::new();

pub fn spawn() -> Handle {
    let (cmd_tx, cmd_rx) = tokio::sync::mpsc::channel(1);
    std::thread::spawn(move || {
        executor(cmd_rx);
    });

    let handle = Handle { cmd_tx };

    // Store the handle globally
    if HTTP_HANDLE.set(handle.clone()).is_err() {
        error!("HTTP handle already initialized");
    }

    handle
}

// Clone the handle because cmd_tx is Clone
impl Clone for Handle {
    fn clone(&self) -> Self {
        Self {
            cmd_tx: self.cmd_tx.clone(),
        }
    }
}

pub fn get_handle() -> Option<&'static Handle> {
    HTTP_HANDLE.get()
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
        let client = reqwest::Client::new();

        while let Some(cmd) = cmd_rx.recv().await {
            match cmd {
                Command::Send { payload } => {
                    match handle_http_request(&client, payload).await {
                        Ok(response) => {
                            trace!("HTTP request successful: {}", response);
                            // Send response back via event system
                            // let response_event = HTTPResponseEvent::success(payload.request_id, response);
                            // let event = HermesEvent::new(response_event, TargetApp::All, TargetModule::All);
                            // crate::event::queue::send(event)?;
                        },
                        Err(e) => {
                            error!("HTTP request failed: {}", e);
                            // Send error response back via event system
                            // let error_event = HTTPResponseEvent::error(payload.request_id, e.to_string());
                            // let event = HermesEvent::new(error_event, TargetApp::All, TargetModule::All);
                            // crate::event::queue::send(event)?;
                        },
                    }
                },
            }
        }
    });
}

async fn handle_http_request(
    client: &reqwest::Client,
    payload: super::Payload,
) -> Result<String, HttpError> {
    let body_str = String::from_utf8(payload.body)
        .map_err(|_| HttpError::InvalidUtf8)?;

    let request_line = body_str.lines().next()
        .ok_or(HttpError::EmptyBody)?;

    let parts: Vec<&str> = request_line.split_whitespace().collect();
    if parts.len() < 2 {
        return Err(HttpError::InvalidRequestLine);
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
    trace!("Making HTTP request to: {}", url);

    let response = if request_line.starts_with("POST") {
        let body_content = body_str.split("\r\n\r\n").last().unwrap_or("");
        client
            .post(&url)
            .body(body_content.to_string())
            .send()
            .await?
    } else {
        client.get(&url).send().await?
    };

    response.text().await.map_err(HttpError::from)
}

