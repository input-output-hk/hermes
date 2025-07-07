use std::{sync::OnceLock, time::Duration};

use tracing::{error, info, trace};

use crate::{
    event::{HermesEvent, HermesEventPayload, TargetApp, TargetModule},
    runtime_extensions::hermes::http_request::Payload,
    wasm::module::ModuleInstance,
};

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
    EventTimeout,
}

impl std::fmt::Display for HttpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HttpError::InvalidUtf8 => write!(f, "Invalid UTF-8 in body"),
            HttpError::EmptyBody => write!(f, "Empty HTTP body"),
            HttpError::InvalidRequestLine => write!(f, "Invalid HTTP request line"),
            HttpError::NetworkError(e) => write!(f, "Network error: {}", e),
            HttpError::EventTimeout => write!(f, "Event response timeout"),
        }
    }
}

impl std::error::Error for HttpError {}

impl From<reqwest::Error> for HttpError {
    fn from(err: reqwest::Error) -> Self {
        HttpError::NetworkError(err)
    }
}

// Custom HTTP response event for this module
#[derive(Debug, Clone)]
pub struct HttpResponseEvent {
    pub request_id: Option<String>,
    pub success: bool,
    pub response_body: String,
    pub error_message: Option<String>,
}

impl HttpResponseEvent {
    pub fn success(request_id: Option<String>, response_body: String) -> Self {
        Self {
            request_id,
            success: true,
            response_body,
            error_message: None,
        }
    }

    pub fn error(request_id: Option<String>, error_message: String) -> Self {
        Self {
            request_id,
            success: false,
            response_body: String::new(),
            error_message: Some(error_message),
        }
    }
}

impl HermesEventPayload for HttpResponseEvent {
    fn event_name(&self) -> &'static str {
        "http_response"
    }

    fn execute(&self, module: &mut ModuleInstance) -> anyhow::Result<()> {
        // Call the WASM module's HTTP response handler
        let response_data = serde_json::json!({
            "request_id": self.request_id,
            "success": self.success,
            "response_body": self.response_body,
            "error_message": self.error_message
        });

        // Access module ID through the runtime context
        let module_id = module.store.data().module_id();
        info!(
            "HTTP Executing HTTP response event for module: {:?}",
            module_id
        );
        info!("HTTP Response data: {}", response_data);

        // TODO: Call the actual WASM module function to handle the HTTP response
        // This might be something like:
        // module.instance.call_http_response_handler(&mut module.store, response_data)?;

        Ok(())
    }
}

#[derive(Clone)]
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

    // Create the handle first
    let handle = Handle { cmd_tx };

    // Store it in the global static immediately
    if HTTP_HANDLE.set(handle.clone()).is_err() {
        error!("HTTP handle already initialized");
    }

    // Now spawn the background thread
    std::thread::spawn(move || {
        executor(cmd_rx);
    });

    handle
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
                    let request_id = payload.request_id.clone();
                    match handle_http_request(&client, payload).await {
                        Ok(response) => {
                            info!("HTTP request successful: {}", response);
                            // Send response back via event system
                            send_http_response_event(HttpResponseEvent::success(
                                request_id, response,
                            ))
                            .await;
                        },
                        Err(e) => {
                            error!("HTTP request failed: {}", e);
                            // Send error response back via event system
                            send_http_error_event(HttpResponseEvent::error(
                                request_id,
                                e.to_string(),
                            ))
                            .await;
                        },
                    }
                },
            }
        }
    });
}

async fn handle_http_request(
    client: &reqwest::Client, payload: super::Payload,
) -> Result<String, HttpError> {
    error!("HTTP {:?} {:?}", payload.host_uri, payload.port);

    let body_str = String::from_utf8(payload.body).map_err(|_| HttpError::InvalidUtf8)?;

    let request_line = body_str.lines().next().ok_or(HttpError::EmptyBody)?;

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
    info!("Making HTTP request to: {}", url);

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

async fn send_http_response_event(response_event: HttpResponseEvent) {
    let event = HermesEvent::new(response_event, TargetApp::All, TargetModule::All);

    if let Err(e) = crate::event::queue::send(event) {
        error!("Failed to send HTTP response event: {}", e);
    } else {
        info!("HTTP response event sent successfully");
    }
}

async fn send_http_error_event(error_event: HttpResponseEvent) {
    let event = HermesEvent::new(error_event, TargetApp::All, TargetModule::All);

    if let Err(e) = crate::event::queue::send(event) {
        error!("Failed to send HTTP error event: {}", e);
    } else {
        info!("HTTP error event sent successfully");
    }
}
