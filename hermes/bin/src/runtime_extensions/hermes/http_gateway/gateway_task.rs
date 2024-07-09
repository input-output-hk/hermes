//! HTTP Gateway task

use std::{
    collections::HashMap,
    convert::Infallible,
    net::SocketAddr,
    sync::{Arc, Mutex},
};

use hyper::{
    self,
    server::{conn::AddrStream, Server},
    service::{make_service_fn, service_fn},
};
use tracing::{error, info};

use super::routing::router;

/// HTTP Gateway port
const GATEWAY_PORT: u16 = 5000;

/// hostname (node name)
#[derive(Debug, Clone)]
pub(crate) struct Hostname(pub String);

/// Config for gateway setup
#[derive(Debug, Clone)]
pub(crate) struct Config {
    /// Valid host names
    pub(crate) valid_hosts: Vec<Hostname>,
    /// Local address for boot strap
    pub(crate) local_addr: SocketAddr,
}

/// We will eventually use env vars when deployment pipeline is in place, hardcoded
/// default is fine for now.
impl Default for Config {
    fn default() -> Self {
        Self {
            valid_hosts: [
                Hostname("hermes.local".to_owned()),
                Hostname("localhost".to_owned()),
            ]
            .to_vec(),
            local_addr: SocketAddr::new([127, 0, 0, 1].into(), GATEWAY_PORT),
        }
    }
}

/// Unique identifier for incoming request
#[derive(Eq, Hash, PartialEq, Clone, Debug)]
pub(crate) struct EventUID(pub String);

/// Incoming request client IP
#[derive(Debug)]
#[allow(dead_code)]
pub(crate) struct ClientIPAddr(pub SocketAddr);

/// Has the event been processed
#[derive(Debug)]
#[allow(dead_code)]
pub(crate) struct Processed(pub bool);

/// Is the connection still live
#[derive(Debug)]
#[allow(dead_code)]
pub(crate) struct LiveConnection(pub bool);

/// Manages and tracks client connections
#[derive(Debug)]
pub(crate) struct ConnectionManager {
    /// Connection metadata
    connection_context: Mutex<HashMap<EventUID, (ClientIPAddr, Processed, LiveConnection)>>,
}

impl ConnectionManager {
    /// Get connection context
    pub fn get_connection_manager_context(
        &self,
    ) -> &Mutex<HashMap<EventUID, (ClientIPAddr, Processed, LiveConnection)>> {
        &self.connection_context
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

    let connection_manager = Arc::new(ConnectionManager {
        connection_context: Mutex::new(HashMap::new()),
    });

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

    rt.block_on(async move {
        let gateway_service = make_service_fn(|client: &AddrStream| {
            let connection_manager = connection_manager.clone();
            let ip = client.remote_addr();
            let config = config.clone();

            async move {
                Ok::<_, Infallible>(service_fn(move |req| {
                    router(req, connection_manager.clone(), ip, config.clone())
                }))
            }
        });

        match Server::bind(&config.local_addr)
            .serve(gateway_service)
            .await
        {
            Ok(()) => (),
            Err(err) => {
                error!("Failing to start HTTP gateway server: {:?}", err);
                error!("Retrying!");
                executor();
            },
        }
    });
}
