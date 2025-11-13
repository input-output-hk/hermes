use std::sync::{Arc, LazyLock};

use rustls::{ClientConfig, RootCertStore, pki_types::ServerName};
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
    sync::oneshot,
};
use tokio_rustls::{TlsConnector, client};
use tracing::{error, trace};
use webpki_roots::TLS_SERVER_ROOTS;

use crate::{
    event::{HermesEvent, TargetApp, TargetModule},
    runtime_extensions::{
        bindings::hermes::http_request::api::{ErrorCode, Payload},
        hermes::http_request::STATE,
    },
};

/// HTTP scheme.
const HTTP: &str = "http://";
/// HTTPS scheme.
const HTTPS: &str = "https://";

/// Represents a command that can be sent to the Tokio runtime task.
pub enum Command {
    /// Command to send an HTTP request.
    Send {
        /// The payload containing the request details.
        payload: Payload,
        /// A channel to send the response back to the caller.
        response_tx: oneshot::Sender<Result<(), ErrorCode>>,
    },
}

/// Represents a sender for commands sent to the Tokio runtime task.
type CommandSender = tokio::sync::mpsc::Sender<Command>;
/// Represents a receiver for commands sent to the Tokio runtime task.
type CommandReceiver = tokio::sync::mpsc::Receiver<Command>;

/// Tokio runtime task handle for sending HTTP requests.
pub struct TokioTaskHandle {
    /// Command sender for sending commands to the Tokio runtime task.
    cmd_tx: CommandSender,
}

impl TokioTaskHandle {
    /// Sends a command to the Tokio runtime task.
    pub fn send(
        &self,
        payload: Payload,
    ) -> Result<(), ErrorCode> {
        let (response_tx, response_rx) = tokio::sync::oneshot::channel();
        self.cmd_tx
            .blocking_send(Command::Send {
                payload,
                response_tx,
            })
            .map_err(|err| {
                tracing::warn!(%err, "failed to send command via channel");
                ErrorCode::Internal
            })?;
        response_rx.blocking_recv().map_err(|err| {
            tracing::warn!(%err, "failed to receive command result via channel");
            ErrorCode::Internal
        })?
    }
}

/// Initializes the Rustls crypto provider for TLS connections.
static INIT_RUSTLS_CRYPTO: LazyLock<Result<(), ErrorCode>> = LazyLock::new(|| {
    rustls::crypto::ring::default_provider()
        .install_default()
        .map_err(|_| {
            tracing::error!("Failed to install default crypto provider for Rustls");
            ErrorCode::Internal
        })
});

/// Initializes the Rustls TLS connector with the default root certificates.
fn init_rustls_connector() -> TlsConnector {
    let mut root_store = RootCertStore::empty();
    root_store.extend(TLS_SERVER_ROOTS.iter().cloned());

    let config = ClientConfig::builder()
        .with_root_certificates(root_store)
        .with_no_client_auth();
    let config = Arc::new(config);
    TlsConnector::from(config)
}

/// Spawns a Tokio runtime task for handling HTTP requests.
pub fn spawn() -> TokioTaskHandle {
    let (cmd_tx, cmd_rx) = tokio::sync::mpsc::channel(1);
    std::thread::spawn(move || {
        executor(cmd_rx);
    });

    TokioTaskHandle { cmd_tx }
}

/// Represents a connection to an HTTP or HTTPS server.
#[allow(clippy::large_enum_variant)]
enum Connection {
    /// Represents a plain HTTP connection.
    Http(TcpStream),
    /// Represents a secure HTTP connection using TLS.
    Https(client::TlsStream<TcpStream>),
}

impl Connection {
    /// Creates a new connection to the specified HTTP or HTTPS server.
    async fn new<S>(
        addr: S,
        port: u16,
    ) -> Result<Self, ErrorCode>
    where
        S: AsRef<str> + Into<String> + core::fmt::Display,
    {
        if let Some(sliced_addr) = addr.as_ref().to_ascii_lowercase().strip_prefix(HTTP) {
            let stream = TcpStream::connect((sliced_addr, port))
                .await
                .map_err(|err| {
                    tracing::debug!(%err, %sliced_addr, "Failed to connect to HTTP server");
                    ErrorCode::HttpConnectionFailed
                })?;
            tracing::trace!(%addr, port, "connected over HTTP");
            return Ok(Connection::Http(stream));
        } else if let Some(sliced_addr) = addr.as_ref().to_ascii_lowercase().strip_prefix(HTTPS) {
            (*INIT_RUSTLS_CRYPTO)?;

            let tcp_stream = TcpStream::connect((sliced_addr, port))
                .await
                .map_err(|err| {
                    tracing::debug!(%err, %sliced_addr, "tcp stream to HTTPs server");
                    ErrorCode::HttpsConnectionFailed
                })?;

            let server_name = ServerName::try_from(sliced_addr.to_string()).map_err(|err| {
                tracing::debug!(%err, %sliced_addr, "invalid server name when connecting to HTTPs server");
                ErrorCode::HttpsConnectionFailed
            })?;

            let stream = STATE
                .tls_connector
                .get_or_init(init_rustls_connector)
                .connect(server_name, tcp_stream)
                .await
                .map_err(|err| {
                    tracing::debug!(%err, %sliced_addr, "TLS handshake failed");
                    ErrorCode::HttpsConnectionFailed
                })?;

            tracing::trace!(%addr, port, "connected over HTTPs");
            return Ok(Connection::Https(stream));
        }
        tracing::debug!(%addr, "missing scheme");
        Err(ErrorCode::MissingScheme)
    }

    /// Sends the HTTP request body and returns the response.
    // TODO[RC]: Timeout or more complex task management needed here
    async fn send(
        &mut self,
        body: &[u8],
    ) -> Result<Vec<u8>, ErrorCode> {
        let mut response = Vec::new();
        match self {
            Connection::Http(tcp_stream) => {
                tcp_stream.write_all(body).await.map_err(|err| {
                    error!("failed to send HTTP request: {err}");
                    ErrorCode::HttpSendFailed
                })?;
                read_to_end_ignoring_unexpected_eof(tcp_stream, &mut response)
                    .await
                    .map_err(|err| {
                        tracing::debug!(%err, "failed to read from HTTP server");
                        ErrorCode::HttpSendFailed
                    })?;
                tracing::debug!(length_bytes = response.len(), "got response");
            },
            Connection::Https(tls_stream) => {
                tls_stream.write_all(body).await.map_err(|err| {
                    tracing::error!(%err, "failed to connect to HTTPs server");
                    ErrorCode::HttpsSendFailed
                })?;
                read_to_end_ignoring_unexpected_eof(tls_stream, &mut response)
                    .await
                    .map_err(|err| {
                        tracing::debug!(%err, "failed to read from HTTPs server");
                        ErrorCode::HttpsSendFailed
                    })?;
                tracing::debug!(length_bytes = response.len(), "got response");
            },
        }
        Ok(response)
    }
}

/// Reads all bytes from the reader until EOF, ignoring `UnexpectedEof` errors.
async fn read_to_end_ignoring_unexpected_eof<R>(
    reader: &mut R,
    buf: &mut Vec<u8>,
) -> std::io::Result<usize>
where
    R: AsyncRead + Unpin,
{
    // TODO[RC]: This won't work for payloads that do not include "Connection: close", we need
    // a more sophisticated processing.
    match reader.read_to_end(buf).await {
        Ok(0) => Ok(0),
        Ok(n) => Ok(n),
        Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
            tracing::debug!("HTTPs connection closed unexpectedly, but no big deal");
            Ok(buf.len())
        },
        Err(e) => Err(e),
    }
}

/// Sends an HTTP request and puts the response event on the queue.
async fn send_http_request(payload: Payload) -> Result<(), ErrorCode> {
    // TODO[RC]: Make sure there are no stray threads left running
    let mut conn = Connection::new(payload.host_uri, payload.port).await?;
    let response = conn.send(&payload.body).await?;

    let on_http_response_event = super::event::OnHttpResponseEvent {
        request_id: payload.request_id,
        response,
    };

    crate::event::queue::send(HermesEvent::new(
        on_http_response_event,
        TargetApp::All,
        TargetModule::All,
    ))
    .map_err(|err| {
        tracing::error!(%err, "queue failure");
        ErrorCode::Internal
    })?;
    Ok(())
}

/// Handles a command sent to the Tokio runtime task.
async fn handle_command(cmd: Command) {
    match cmd {
        Command::Send {
            payload,
            response_tx,
        } => {
            let sending_result = send_http_request(payload).await;
            let _ = response_tx.send(sending_result);
        },
    }
}

/// Tokio runtime task for handling HTTP requests.
fn executor(mut cmd_rx: CommandReceiver) {
    let res = tokio::runtime::Builder::new_current_thread()
        .enable_io()
        .build();

    let rt = match res {
        Ok(rt) => rt,
        Err(err) => {
            error!(error = ?err, "failed to start Http Request Runtime Extension background thread");
            return;
        },
    };

    trace!("Created Tokio runtime for Http Request Runtime Extension");

    rt.block_on(async move {
        while let Some(cmd) = cmd_rx.recv().await {
            tokio::spawn(handle_command(cmd));
        }
    });
}
