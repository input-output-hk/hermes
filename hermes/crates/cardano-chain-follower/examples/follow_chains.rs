//! This example shows how to use the chain follower to follow all chains, until they have all reached tip.
//! It will report on how many blocks for each chain exist between eras, and also how long each chain took to reach its tip.

// Allowing since this is example code.
#![allow(clippy::unwrap_used)]

use std::{error::Error, time::Duration};

use cardano_chain_follower::{
    ChainFollower, ChainSyncConfig, ChainUpdate, Network, Point, PointOrTip,
};
use clap::{arg, ArgAction, Command};
use tokio::time::sleep;
use tracing::{debug, error, info, level_filters::LevelFilter};
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
async fn start_sync_for(network: &Network) -> Result<(), Box<dyn Error>> {
    let cfg = ChainSyncConfig::default_for(*network);
    info!(chain = cfg.chain.to_string(), "Starting Sync");

    if let Err(error) = cfg.run().await {
        error!("Failed to start sync task for {} : {}", network, error);
        Err(error)?;
    }

    Ok(())
}

/// Try and follow a chain continuously, from Genesis until Tip.
#[allow(clippy::panic)]
async fn follow_for(network: Network) {
    //loop {
    info!(chain = network.to_string(), "Following");
    let mut follower =
        ChainFollower::new(network, PointOrTip::Point(Point::Origin), PointOrTip::Tip).await;

    let mut current_era = String::new();
    let mut last_update: Option<ChainUpdate> = None;
    let mut prev_hash: Option<pallas_crypto::hash::Hash<32>> = None;
    let mut last_immutable: bool = false;
    let mut reached_tip = false; // After we reach TIP we show all block we process.

    while let Some(chain_update) = follower.next().await {
        if chain_update.tip {
            reached_tip = true;
        }

        let block = chain_update.block_data().decode();
        let this_era = block.era().to_string();
        if (current_era != this_era) || (chain_update.immutable() != last_immutable) || reached_tip
        {
            current_era = this_era;
            last_immutable = chain_update.immutable();
            info!(chain = network.to_string(), "{}", chain_update);
        }

        let this_prev_hash = match block {
            pallas::ledger::traverse::MultiEraBlock::EpochBoundary(ref block) => {
                Some(block.header.prev_block)
            },
            pallas::ledger::traverse::MultiEraBlock::AlonzoCompatible(ref block, _) => {
                block.header.header_body.prev_hash
            },
            pallas::ledger::traverse::MultiEraBlock::Babbage(ref block) => {
                block.header.header_body.prev_hash
            },
            pallas::ledger::traverse::MultiEraBlock::Byron(ref block) => {
                Some(block.header.prev_block)
            },
            pallas::ledger::traverse::MultiEraBlock::Conway(ref block) => {
                block.header.header_body.prev_hash
            },
            _ => None,
        };
        if last_update.is_some() && prev_hash != this_prev_hash {
            debug!("last_update = {}", last_update.unwrap());
            debug!("prev_hash = {:?}", prev_hash);
            debug!("this_prev_hash = {:?}", this_prev_hash);
            error!(
                chain = network.to_string(),
                "Chain is broken: {}", chain_update
            );
            panic!("DEAD");
        }

        prev_hash = Some(block.hash());
        last_update = Some(chain_update);
    }

    if let Some(last_update) = last_update {
        info!(chain = network.to_string(), "Last Update: {}", last_update);
    }
    info!(chain = network.to_string(), "Following Completed.");

    //}
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt()
        .with_file(true)
        .with_line_number(true)
        .with_thread_names(true)
        .with_thread_ids(true)
        .pretty()
        .with_env_filter(
            EnvFilter::builder()
                .with_default_directive(LevelFilter::DEBUG.into())
                .from_env_lossy(),
        )
        .init();

    let networks = process_argument();
    let parallelism = std::thread::available_parallelism()?;
    info!(
        Parallelism = parallelism,
        "Cardano Chain Followers Starting."
    );

    // First we need to actually start the underlying sync tasks for each blockchain.
    for network in &networks {
        start_sync_for(network).await?;
    }

    // Make a follower for the network.
    let mut tasks = Vec::new();
    for network in &networks {
        tasks.push(tokio::spawn(follow_for(*network)));
    }

    // Wait forever (a really really long time anyway)
    sleep(Duration::from_secs(u64::MAX)).await;

    Ok(())
}
