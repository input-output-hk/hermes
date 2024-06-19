//! This example shows how to use the chain follower to follow all chains, until they have all reached tip.
//! It will report on how many blocks for each chain exist between eras, and also how long each chain took to reach its tip.

// Allowing since this is example code.
#![allow(clippy::unwrap_used)]

use std::{error::Error, time::Duration};

use cardano_chain_follower::{ChainSyncConfig, Network};
use tokio::time::sleep;
use tracing::{error, level_filters::LevelFilter};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::builder()
                .with_default_directive(LevelFilter::DEBUG.into())
                .from_env_lossy(),
        )
        .init();

    // First we need to actually start the underlying sync tasks for each blockchain.

    // Start the sync task for preprod.
    if let Err(error) = ChainSyncConfig::default_for(Network::Preprod).run().await {
        error!(
            "Failed to start sync task for {} : {}",
            Network::Preprod,
            error
        );
        Err(error)?;
    }

    // Wait forever (a really really long time anyway)
    sleep(Duration::from_secs(u64::MAX)).await;

    Ok(())
}
