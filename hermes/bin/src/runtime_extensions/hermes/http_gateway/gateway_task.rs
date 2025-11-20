//! HTTP Gateway task

use std::{net::SocketAddr, sync::Arc};

use dashmap::DashMap;
use hyper::{self, service::service_fn};
use hyper_util::{
    rt::{TokioExecutor, TokioIo},
    server::conn::auto::Builder,
};
use tokio::net::TcpListener;
#[allow(unused_imports, reason = "`debug` used in debug builds.")]
use tracing::{debug, error, info};

use super::routing::router;

/// HTTP Gateway port
const GATEWAY_PORT: u16 = 5000;

/// hostname (node name)
#[cfg_attr(debug_assertions, derive(Debug))]
#[derive(Clone)]
pub(crate) struct Hostname(pub String);

/// Config for gateway setup
#[cfg_attr(debug_assertions, derive(Debug))]
#[derive(Clone)]
pub(crate) struct Config {
    /// Valid host names
    pub(crate) valid_hosts: Vec<Hostname>,
    /// Local address for boot strap
    pub(crate) local_addr: SocketAddr,
    /// Whether auth is activated
    pub(crate) is_auth_activate: bool,
}

/// We will eventually use env vars when deployment pipeline is in place, hardcoded
/// default is fine for now.
impl Default for Config {
    fn default() -> Self {
        let is_auth_activate = std::env::var("HERMES_AUTH_ACTIVATE")
            .ok()
            .and_then(|v| v.parse::<bool>().ok())
            .unwrap_or(true); // fallback default

        Self {
            valid_hosts: [
                Hostname("hermes.local".to_owned()),
                Hostname("localhost".to_owned()),
            ]
            .to_vec(),
            local_addr: SocketAddr::new([127, 0, 0, 1].into(), GATEWAY_PORT),
            is_auth_activate,
        }
    }
}

/// Unique identifier for incoming request
#[cfg_attr(debug_assertions, derive(Debug))]
#[derive(Eq, Hash, PartialEq, Clone)]
pub(crate) struct EventUID(pub String);

/// Incoming request client IP
#[cfg_attr(debug_assertions, derive(Debug))]
#[derive(Clone)]
#[allow(dead_code)]
pub(crate) struct ClientIPAddr(pub SocketAddr);

/// Has the event been processed
#[cfg_attr(debug_assertions, derive(Debug))]
#[derive(Clone)]
#[allow(dead_code)]
pub(crate) struct Processed(pub bool);

/// Is the connection still live
#[cfg_attr(debug_assertions, derive(Debug))]
#[derive(Clone)]
#[allow(dead_code)]
pub(crate) struct LiveConnection(pub bool);

/// Information about an individual client connection.
type ClientConnectionInfo = (ClientIPAddr, Processed, LiveConnection);

/// Manages and tracks client connections
#[cfg_attr(debug_assertions, derive(Debug))]
#[derive(Clone)]
pub(crate) struct ConnectionManager {
    /// Connection metadata
    connection_context: Arc<DashMap<EventUID, ClientConnectionInfo>>,
}

impl std::fmt::Display for ConnectionManager {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        write!(
            f,
            "{} Concurrent Connections to the ConnectionManager.",
            self.connection_context.len()
        )
    }
}

impl ConnectionManager {
    /// Create a new Connection Manager.
    pub(crate) fn new() -> Self {
        Self {
            connection_context: Arc::new(DashMap::new()),
        }
    }

    /// Insert into the Connection Manager.
    pub(crate) fn insert(
        &self,
        key: EventUID,
        value: ClientConnectionInfo,
    ) -> Option<ClientConnectionInfo> {
        let result = self.connection_context.insert(key, value);
        #[cfg(debug_assertions)]
        debug!(result=?result, connection_context = ?self.connection_context, "Connection Manager Inserted.");
        result
    }
}

/// Spawns a OS thread running the Tokio runtime task.
pub(crate) fn spawn() {
    std::thread::spawn(move || {
        executor();
    });
}

/// Starts the HTTP Gateway
fn executor() {
    let config = Config::default();

    let connection_manager = ConnectionManager::new();

    let res = tokio::runtime::Builder::new_current_thread()
        .enable_io()
        .enable_time()
        .build();

    let rt = match res {
        Ok(rt) => rt,
        Err(err) => {
            error!(error = ?err, "Failed to start HTTP gateway background thread");
            return;
        },
    };

    info!("Starting HTTP Gateway");

    let tokio_executor = TokioExecutor::new();

    rt.block_on(async move {
        let listener = match TcpListener::bind(&config.local_addr).await {
            Ok(listener) => listener,
            Err(err) => {
                error!("Bind to {} failed: {:?}", config.local_addr, err);
                return;
            },
        };

        loop {
            let (stream, remote_addr) = match listener.accept().await {
                Ok(conn) => conn,
                Err(err) => {
                    error!("Accept failed: {:?}", err);
                    continue;
                },
            };

            let connection_manager = connection_manager.clone();
            let config = config.clone();
            let tokio_executor = tokio_executor.clone();

            tokio::spawn(async move {
                let io = TokioIo::new(stream);

                let service = service_fn(move |req| {
                    router(req, connection_manager.clone(), remote_addr, config.clone())
                });

                if let Err(err) = Builder::new(tokio_executor)
                    .serve_connection(io, service)
                    .await
                {
                    error!("Failed to serve HTTP connection: {:?}", err);
                    // Don't call executor() recursively - just let this connection fail
                    // The main loop will continue accepting new connections
                }
            });
        }
    });
}
