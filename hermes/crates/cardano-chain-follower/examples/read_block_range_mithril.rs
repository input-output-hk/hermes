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

    let data_vec = follower
        .read_block_range(
            // Block: 1794556
            Point::Specific(
                49_075_380,
                hex::decode("a5d7ffbc7e61bf19e90b2b07276026d5fdd43424cc3436547b9532ca4a9f19ad")?,
            ),
            // Block: 1794560
            Point::Specific(
                49_075_522,
                hex::decode("b7639b523f320643236ab0fc04b7fd381dedd42c8d6b6433b5965a5062411396")?,
            ),
        )
        .read()
        .await?;

    for data in data_vec {
        let block = data.decode()?;

        println!(
            "Block {} has {} transactions",
            block.number(),
            block.tx_count()
        );
    }

    Ok(())
}
