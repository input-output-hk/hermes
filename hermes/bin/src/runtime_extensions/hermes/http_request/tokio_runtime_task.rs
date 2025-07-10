use std::{
    io::{Read, Write},
    net::TcpStream,
    sync::Arc,
};

use rustls::{pki_types::ServerName, ClientConfig, ClientConnection, RootCertStore, StreamOwned};
use thiserror::Error;
use tokio::sync::{mpsc, oneshot};
use tracing::{error, trace};
use webpki_roots::TLS_SERVER_ROOTS;

use crate::{
    event::{HermesEvent, HermesEventPayload, TargetApp, TargetModule},
    runtime_extensions::bindings::hermes::http_request::api::{ErrorCode, Payload},
};

const HTTP: &str = "http://";
const HTTPS: &str = "https://";

pub enum Command {
    Send {
        payload: Payload,
        sender: oneshot::Sender<bool>,
    },
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

impl From<RequestSendingError> for ErrorCode {
    fn from(value: RequestSendingError) -> Self {
        match value {
            // We map all "internal" errors to `ErrorCode::Internal` to not expose implementation
            // details to the user. Detailed information will be available in logs.
            RequestSendingError::ChannelSend(_) | RequestSendingError::ResponseReceive(_) => {
                ErrorCode::Internal
            },
        }
    }
}

impl TokioTaskHandle {
    /// Sends a command to the Tokio runtime task.
    pub fn send(&self, payload: Payload) -> Result<(), RequestSendingError> {
        let (sender, receiver) = tokio::sync::oneshot::channel();
        self.cmd_tx
            .blocking_send(Command::Send { payload, sender })?;
        receiver.blocking_recv()?;
        Ok(())
    }
}

pub fn spawn() -> TokioTaskHandle {
    let (cmd_tx, cmd_rx) = tokio::sync::mpsc::channel(1);
    std::thread::spawn(move || {
        executor(cmd_rx);
    });

    TokioTaskHandle { cmd_tx }
}

// TODO[RC]: Use Tokio here
enum Connection {
    Http(TcpStream),
    Https(StreamOwned<ClientConnection, TcpStream>),
}

impl Connection {
    fn new<S>(addr: S, port: u16) -> Result<Self, ErrorCode>
    where S: AsRef<str> + Into<String> + core::fmt::Display {
        if addr.as_ref().starts_with(HTTP) {
            let sliced_addr = &addr.as_ref()[HTTP.len()..];
            let stream = TcpStream::connect((sliced_addr, port)).map_err(|err| {
                // TODO[RC]: These should be debug!, but debug do not show up even with
                // RUST_LOG=debug - to be investigated.
                tracing::error!(%err, %sliced_addr, "Failed to connect to HTTP server");
                ErrorCode::HttpConnectionFailed
            })?;
            tracing::debug!(%addr, port, "connected over HTTP");
            return Ok(Connection::Http(stream));
        } else if addr.as_ref().starts_with(HTTPS) {
            // TODO[RC]: No need to configure RootCertStore for every connection
            let sliced_addr = &addr.as_ref()[HTTPS.len()..];
            let mut root_store = RootCertStore::empty();
            root_store.extend(TLS_SERVER_ROOTS.iter().cloned());
            let config = ClientConfig::builder()
                .with_root_certificates(root_store)
                .with_no_client_auth();
            let config = Arc::new(config);
            let tcp = TcpStream::connect((sliced_addr.as_ref(), port)).map_err(|err| {
                tracing::error!(%err, %sliced_addr, "Failed to connect to HTTPs server");
                ErrorCode::HttpsConnectionFailed
            })?;
            let server_name = ServerName::try_from(sliced_addr.to_string()).map_err(|err| {
                tracing::error!(%err, %sliced_addr, "Failed to connect to HTTPs server");
                ErrorCode::HttpsConnectionFailed
            })?;
            let conn = ClientConnection::new(config, server_name).map_err(|err| {
                tracing::error!(%err, %sliced_addr, "Failed to connect to HTTPs server");
                ErrorCode::HttpsConnectionFailed
            })?;
            let mut stream = StreamOwned::new(conn, tcp);
            tracing::debug!(%addr, port, "connected over HTTPs");
            return Ok(Connection::Https(stream));
        } else {
            tracing::debug!(%addr, "missing scheme");
            return Err(ErrorCode::MissingScheme);
        }
    }

    // TODO[RC]: Timeout needed here, but maybe first switch to Tokio
    fn send(&mut self, body: &[u8]) -> Result<Vec<u8>, ErrorCode> {
        let mut response = Vec::new();
        match self {
            Connection::Http(ref mut tcp_stream) => {
                tcp_stream.write_all(body).map_err(|err| {
                    error!("failed to send HTTP request: {err}");
                    ErrorCode::HttpSendFailed
                })?;
                tracing::debug!("request sent, awaiting response");
                read_to_end_ignoring_unexpected_eof(tcp_stream, &mut response).map_err(|err| {
                    tracing::debug!(%err, "failed to read from HTTP server");
                    ErrorCode::HttpsSendFailed
                });
                tracing::debug!(length_bytes = response.len(), "got response");
            },
            Connection::Https(tls_stream) => {
                tls_stream.write_all(body).map_err(|err| {
                    tracing::error!(%err, "failed to connect to HTTPs server");
                    ErrorCode::HttpsSendFailed
                })?;
                read_to_end_ignoring_unexpected_eof(tls_stream, &mut response).map_err(|err| {
                    tracing::debug!(%err, "failed to read from HTTPs server");
                    ErrorCode::HttpsSendFailed
                });
                tracing::debug!(length_bytes = response.len(), "got response");
            },
        }
        Ok(response)
    }
}

fn read_to_end_ignoring_unexpected_eof<R>(
    reader: &mut R, buf: &mut Vec<u8>,
) -> std::io::Result<usize>
where R: Read {
    match reader.read_to_end(buf) {
        Ok(0) => Ok(0),
        Ok(n) => Ok(n),
        Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
            tracing::debug!("HTTPs connection closed unexpectedly, but no big deal");
            Ok(buf.len())
        },
        Err(e) => Err(e),
    }
}

fn send_request_in_background(payload: Payload) -> bool {
    // TODO[RC]: Make sure there are no stray threads left running
    std::thread::spawn(move || {
        let mut conn = Connection::new(payload.host_uri, payload.port).unwrap();
        let response = conn.send(&payload.body);
        match response {
            Ok(response) => {
                let on_http_response_event = super::event::OnHttpResponseEvent {
                    request_id: payload.request_id,
                    response,
                };

                crate::event::queue::send(HermesEvent::new(
                    on_http_response_event,
                    TargetApp::All,
                    TargetModule::All,
                ));
            },
            Err(err) => tracing::debug!(%err, "error sending request"),
        }
    });

    true
}

fn executor(mut cmd_rx: CommandReceiver) {
    let res = tokio::runtime::Builder::new_current_thread().build();

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
                Command::Send { payload, sender } => {
                    let sending_result = send_request_in_background(payload);
                    let _ = sender.send(sending_result);
                },
            }
        }
    });
}
