//! This example shows how to use the chain reader to read arbitrary blocks
//! from Mithril snapshot files.

// Allowing since this is example code.
#![allow(clippy::unwrap_used)]

use std::{error::Error, path::PathBuf};

use cardano_chain_follower::{Follower, FollowerConfigBuilder, Network, Point};
use tracing::level_filters::LevelFilter;
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
    let config = FollowerConfigBuilder::default()
        .mithril_snapshot_path(
            PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap())
                .join("examples/snapshot_data"),
        )
        .build();

    let follower = Follower::connect(
        "preprod-node.play.dev.cardano.org:3001",
        Network::Preprod,
        config,
    )
    .await?;

    let data = follower
        .read_block(Point::Specific(
            49_075_418,
            hex::decode("bdb5ce7788850c30342794f252b1d955086862e8f7cb90a32a8f560b693ca78a")?,
        ))
        .read()
        .await?;

    let block = data.decode()?;

    let total_fee = block
        .txs()
        .iter()
        .map(|tx| tx.fee().unwrap_or_default())
        .sum::<u64>();

    println!("Block number: {}", block.number());
    println!("Total fee: {total_fee}");

    Ok(())
}
