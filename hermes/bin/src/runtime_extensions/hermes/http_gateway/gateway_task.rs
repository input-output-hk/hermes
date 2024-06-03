//! HTTP Gateway task

use std::{
    collections::HashMap,
    convert::Infallible,
    sync::{Arc, Mutex},
};

use hyper::{
    self,
    server::{conn::AddrStream, Server},
    service::{make_service_fn, service_fn},
};
use tracing::{error, info};

use super::routing::router;

struct ConnectionManager {
    pub connection_context: Mutex<HashMap<String, String>>,
}

/// Spawns a OS thread running the Tokio runtime task.
pub fn spawn() {
    std::thread::spawn(move || {
        executor();
    });
}

/// Starts the HTTP Gateway
fn executor() {
    let shared = Arc::new(ConnectionManager {
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
        let addr = ([127, 0, 0, 1], 5000).into();

        let gateway_service = make_service_fn(|client: &AddrStream| {
            let connection_manager = shared.clone();

            match connection_manager.connection_context.try_lock() {
                Ok(mut context) => {
                    context.insert(
                        client.remote_addr().to_string(),
                        rusty_ulid::generate_ulid_string(),
                    );
                },
                Err(err) => error!(
                    "Unable to record connection state for {:?} {:?}",
                    client.remote_addr().to_string(),
                    err
                ),
            }

            async move { Ok::<_, Infallible>(service_fn(router)) }
        });

        Server::bind(&addr)
            .serve(gateway_service)
            .await
            .expect("Failing to start HTTP gateway server is not recoverable");
    });
}
