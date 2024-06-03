//! HTTP Gateway task

use std::convert::Infallible;

use hyper::{
    server::Server,
    service::{make_service_fn, service_fn},
    {self},
};
use tracing::{error, info};

use super::routing::router;

/// Spawns a OS thread running the Tokio runtime task.
pub fn spawn() {
    std::thread::spawn(move || {
        executor();
    });
}

/// Starts the HTTP Gateway
fn executor() {
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
        let gateway_service =
            make_service_fn(|_| async { Ok::<_, Infallible>(service_fn(router)) });
        Server::bind(&addr)
            .serve(gateway_service)
            .await
            .expect("Failing to start HTTP gateway server is not recoverable");
    });
}
