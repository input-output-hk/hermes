//! This example shows how to use the chain follower to follow all chains, until they have all reached tip.
//! It will report on how many blocks for each chain exist between eras, and also how long each chain took to reach its tip.

// Allowing since this is example code.
#![allow(clippy::unwrap_used)]

use std::{error::Error, time::Duration};

use cardano_chain_follower::{ChainSyncConfig, Network};
use clap::{arg, ArgAction, Command};
use tokio::time::sleep;
use tracing::{error, level_filters::LevelFilter};
use tracing_subscriber::EnvFilter;

/// Process our CLI Arguments
fn process_argument() -> Vec<Network> {
    let matches = Command::new("follow_chains")
        .args(&[
            arg!(--preprod "Follow Preprod network").action(ArgAction::SetTrue),
            arg!(--preview "Follow Preview network").action(ArgAction::SetTrue),
            arg!(--mainnet "Follow Mainnet network").action(ArgAction::SetTrue),
            arg!(--all "Follow All networks").action(ArgAction::SetTrue),
        ])
        .get_matches();

    let mut networks = vec![];
    if matches.get_flag("preprod") || matches.get_flag("all") {
        networks.push(Network::Preprod);
    }
    if matches.get_flag("preview") || matches.get_flag("all") {
        networks.push(Network::Preview);
    }
    if matches.get_flag("mainnet") || matches.get_flag("all") {
        networks.push(Network::Mainnet);
    }

    networks
}

/// Start syncing a particular network
async fn start_sync_for(network: Network) -> Result<(), Box<dyn Error>> {
    if let Err(error) = ChainSyncConfig::default_for(network).run().await {
        error!("Failed to start sync task for {} : {}", network, error);
        Err(error)?;
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::builder()
                .with_default_directive(LevelFilter::DEBUG.into())
                .from_env_lossy(),
        )
        .init();

    let networks = process_argument();

    // First we need to actually start the underlying sync tasks for each blockchain.
    for network in networks {
        start_sync_for(network).await?;
    }

    // Wait forever (a really really long time anyway)
    sleep(Duration::from_secs(u64::MAX)).await;

    Ok(())
}
