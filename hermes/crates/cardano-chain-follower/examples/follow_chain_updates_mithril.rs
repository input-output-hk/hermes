//! This example shows how to use the chain follower to follow chain updates on
//! a Cardano network chain.

// Allowing since this is example code.
#![allow(clippy::unwrap_used)]

use std::{error::Error, path::PathBuf};

use cardano_chain_follower::{ChainUpdate, FollowerConfigBuilder, Network, Point};
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

    // Create a follower config specifying the Mithril snapshot path and
    // to follow from block 1794552 (preprod).
    let mut follower = FollowerConfigBuilder::default_for(Network::Preprod)
        .follow_from(Point::Specific(
            49_075_262,
            hex::decode("e929cd1bf8ec78844ec9ea450111aaf55fbf17540db4b633f27d4503eebf2218")?,
        ))
        .mithril_snapshot_path(
            PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap())
                .join("examples/snapshot_data"),
            false,
        )
        .build()
        .connect()
        .await?;

    // Wait for some chain updates and shutdown.
    for _ in 0..10 {
        let chain_update = follower.next().await?;

        match chain_update {
            ChainUpdate::Block(data) => {
                let block = data.decode()?;

                println!(
                    "New block NUMBER={} SLOT={} HASH={}",
                    block.number(),
                    block.slot(),
                    hex::encode(block.hash()),
                );
            },
            ChainUpdate::Rollback(data) => {
                let block = data.decode()?;

                println!(
                    "Rollback block NUMBER={} SLOT={} HASH={}",
                    block.number(),
                    block.slot(),
                    hex::encode(block.hash()),
                );
            },
        }
    }

    // Waits for the follower background task to exit.
    follower.close().await?;

    Ok(())
}
