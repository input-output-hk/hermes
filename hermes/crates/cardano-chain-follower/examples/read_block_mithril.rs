//! This example shows how to use the chain follower to read arbitrary blocks
//! from Mithril snapshot files.

// Allowing since this is example code.
#![allow(clippy::unwrap_used)]

use std::{error::Error, path::PathBuf};

use cardano_chain_follower::{FollowerConfigBuilder, Network, Point};
use tracing::{info, level_filters::LevelFilter};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::builder()
                .with_default_directive(LevelFilter::INFO.into())
                .from_env_lossy(),
        )
        .init();

    // Defaults to start following from the tip.
    let follower = FollowerConfigBuilder::default_for(Network::Preprod)
        .mithril_snapshot_path(
            PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap())
                .join("examples/snapshot_data"),
            false,
        )
        .build()
        .connect()
        .await?;

    let data = follower
        .read_block(Point::Specific(
            49_075_418,
            hex::decode("bdb5ce7788850c30342794f252b1d955086862e8f7cb90a32a8f560b693ca78a")?,
        ))
        .await?;

    let block = data.decode()?;

    let total_fee = block
        .txs()
        .iter()
        .map(|tx| tx.fee().unwrap_or_default())
        .sum::<u64>();

    info!("Block number: {}", block.number());
    info!("Total fee: {total_fee}");

    Ok(())
}
