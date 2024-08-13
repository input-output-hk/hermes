//! This example shows how to use the chain follower to follow all chains, until they have
//! all reached tip. It will report on how many blocks for each chain exist between eras,
//! and also how long each chain took to reach its tip.

// Allowing since this is example code.
//#![allow(clippy::unwrap_used)]

#[cfg(feature = "mimalloc")]
use mimalloc::MiMalloc;

/// Use Mimalloc for the global allocator.
#[cfg(feature = "mimalloc")]
#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

use std::error::Error;

use cardano_chain_follower::{
    ChainFollower, ChainSyncConfig, ChainUpdate, Kind, Metadata, Network, Statistics, ORIGIN_POINT,
    TIP_POINT,
};
use clap::{arg, ArgAction, ArgMatches, Command};
use tokio::time::Instant;
use tracing::{error, info, level_filters::LevelFilter};
use tracing_subscriber::EnvFilter;

/// Process our CLI Arguments
fn process_argument() -> (Vec<Network>, ArgMatches) {
    let matches = Command::new("follow_chains")
        .args(&[
            arg!(--preprod "Follow Preprod network").action(ArgAction::SetTrue),
            arg!(--preview "Follow Preview network").action(ArgAction::SetTrue),
            arg!(--mainnet "Follow Mainnet network").action(ArgAction::SetTrue),
            arg!(--all "Follow All networks").action(ArgAction::SetTrue),
            arg!(--"stop-at-tip" "Stop when the tip of the blockchain is reached.")
                .action(ArgAction::SetTrue),
            arg!(--"all-live-blocks" "Show all live blocks.").action(ArgAction::SetTrue),
            arg!(--"all-tip-blocks" "Show all blocks read from the Peer as TIP.")
                .action(ArgAction::SetTrue),
            arg!(--"halt-on-error" "Stop the process when an error occurs without retrying.")
                .action(ArgAction::SetTrue),
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

    (networks, matches)
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

/// The interval between showing a block, even if nothing else changed.
const RUNNING_UPDATE_INTERVAL: u64 = 100_000;

/// Try and follow a chain continuously, from Genesis until Tip.
#[allow(clippy::too_many_lines)]
async fn follow_for(network: Network, matches: ArgMatches) {
    info!(chain = network.to_string(), "Following");
    let mut follower = ChainFollower::new(network, ORIGIN_POINT, TIP_POINT).await;

    let all_tip_blocks = matches.get_flag("all-tip-blocks");
    let all_live_blocks = matches.get_flag("all-live-blocks");
    let stop_at_tip = matches.get_flag("stop-at-tip");
    let halt_on_error = matches.get_flag("halt-on-error");

    let mut current_era = String::new();
    let mut last_update: Option<ChainUpdate> = None;
    let mut last_update_shown = false;
    let mut prev_hash: Option<pallas_crypto::hash::Hash<32>> = None;
    let mut last_immutable: bool = false;
    let mut reached_tip = false; // After we reach TIP we show all block we process.
    let mut updates: u64 = 0;
    let mut last_fork = 0;
    let mut follow_all = false;

    let mut last_metrics_time = Instant::now();

    while let Some(chain_update) = follower.next().await {
        updates += 1;

        if chain_update.tip {
            reached_tip = true;
        }

        let block = chain_update.block_data().decode();
        let this_era = block.era().to_string();

        // When we transition between important points, show the last block as well.
        if ((current_era != this_era)
            || (chain_update.immutable() != last_immutable)
            || (last_fork != chain_update.data.fork()))
            && !last_update_shown
        {
            if let Some(last_update) = last_update.clone() {
                info!(
                    chain = network.to_string(),
                    "Chain Update {}:{}",
                    updates - 1,
                    last_update
                );
            }
        }

        // If these become true, we will show all blocks from the follower.
        follow_all = follow_all
            || (!chain_update.immutable() && all_live_blocks)
            || ((chain_update.data.fork() > 1) && all_tip_blocks);

        // Don't know if this update will show or not, so say it didn't.
        last_update_shown = false;

        if (current_era != this_era)
            || (chain_update.immutable() != last_immutable)
            || reached_tip
            || follow_all
            || (updates % RUNNING_UPDATE_INTERVAL == 0)
            || (last_fork != chain_update.data.fork())
        {
            current_era = this_era;
            last_immutable = chain_update.immutable();
            last_fork = chain_update.data.fork();
            info!(
                chain = network.to_string(),
                "Chain Update {updates}:{}", chain_update
            );
            // We already showed the last update, no need to show it again.
            last_update_shown = true;
        }

        let this_prev_hash = block.header().previous_hash();

        // We have no state, so can only check consistency with block updates.
        // But thats OK, the chain follower itself is also checking chain consistency.
        // This is just an example.
        if chain_update.kind == Kind::Block && last_update.is_some() && prev_hash != this_prev_hash
        {
            let display_last_update = if let Some(last_update) = last_update.clone() {
                format!("{last_update}")
            } else {
                "This Can't Happen".to_string()
            };
            error!(
                chain = network.to_string(),
                "Chain is broken: {chain_update} Does not follow: {display_last_update}",
            );
            break;
        }

        // Inspect the transactions in the block.
        let mut dump_raw_aux_data = false;
        for (tx_idx, _tx) in block.txs().iter().enumerate() {
            if let Some(decoded_metadata) = chain_update
                .data
                .txn_metadata(tx_idx, Metadata::cip36::LABEL)
            {
                if !last_update_shown {
                    info!(
                        chain = network.to_string(),
                        "Chain Update {updates}:{}", chain_update
                    );
                    // We already showed the last update, no need to show it again.
                    last_update_shown = true;
                }

                let raw_size = match chain_update
                    .data
                    .txn_raw_metadata(tx_idx, Metadata::cip36::LABEL)
                {
                    Some(raw) => format!("Raw Size: {}", raw.len()),
                    None => "Error: Cip36 Raw Metadata is missing".to_string(),
                };

                #[allow(irrefutable_let_patterns)] // Won't always be irrefutable.
                if let Metadata::DecodedMetadataValues::Cip36(cip36) = &decoded_metadata.value {
                    if !cip36.signed {
                        dump_raw_aux_data = true;
                    }
                }

                info!(
                    chain = network.to_string(),
                    "Cip36 {tx_idx}:{:?} - {raw_size}", decoded_metadata
                );
            }
        }

        if dump_raw_aux_data {
            if let Some(x) = block.as_alonzo() {
                info!(
                    chain = network.to_string(),
                    "Raw Aux Data: {:02x?}", x.auxiliary_data_set
                );
            } else if let Some(x) = block.as_babbage() {
                info!(
                    chain = network.to_string(),
                    "Raw Aux Data: {:02x?}", x.auxiliary_data_set
                );
            } else if let Some(x) = block.as_conway() {
                info!(
                    chain = network.to_string(),
                    "Raw Aux Data: {:02x?}", x.auxiliary_data_set
                );
            }
        }

        prev_hash = Some(block.hash());
        last_update = Some(chain_update);

        if reached_tip && stop_at_tip {
            break;
        }

        let check_time = Instant::now();
        if check_time.duration_since(last_metrics_time).as_secs() >= 60 {
            last_metrics_time = check_time;

            let stats = Statistics::new(network);

            info!("Json Metrics:  {}", stats.as_json(true));

            if halt_on_error
                && (stats.mithril.download_or_validation_failed > 0
                    || stats.mithril.failed_to_get_tip > 0
                    || stats.mithril.tip_did_not_advance > 0
                    || stats.mithril.tip_failed_to_send_to_updater > 0
                    || stats.mithril.failed_to_activate_new_snapshot > 0)
            {
                break;
            }
        }
    }

    if !last_update_shown {
        if let Some(last_update) = last_update.clone() {
            info!(chain = network.to_string(), "Last Update: {}", last_update);
        }
    }

    let stats = Statistics::new(network);
    info!("Json Metrics:  {}", stats.as_json(true));

    info!(chain = network.to_string(), "Following Completed.");
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
                .with_default_directive(LevelFilter::INFO.into())
                .from_env_lossy(),
        )
        .init();

    let (networks, matches) = process_argument();
    let parallelism = std::thread::available_parallelism()?;
    info!(
        Parallelism = parallelism,
        "Cardano Chain Followers Starting."
    );

    #[cfg(feature = "mimalloc")]
    info!("mimalloc global allocator: enabled");

    // First we need to actually start the underlying sync tasks for each blockchain.
    for network in &networks {
        start_sync_for(network).await?;
    }

    // Make a follower for the network.
    let mut tasks = Vec::new();
    for network in &networks {
        tasks.push(tokio::spawn(follow_for(*network, matches.clone())));
    }

    // Wait for all followers to finish.
    for task in tasks {
        task.await?;
    }

    // Keep running for 1 minute after last follower reaches its tip.
    tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;

    Ok(())
}
