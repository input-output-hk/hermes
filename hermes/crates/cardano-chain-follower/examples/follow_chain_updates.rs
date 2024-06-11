//! This example shows how to use the chain follower to follow chain updates on
//! a Cardano network chain.

use std::error::Error;

use cardano_chain_follower::{ChainUpdate, FollowerConfigBuilder, Network};
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
    let mut follower = FollowerConfigBuilder::default_for(Network::Mainnet)
        .build()
        .connect()
        .await?;

    // Wait for 3 chain updates and shutdown.
    for _ in 0..3 {
        let chain_update = follower.next().await?;

        match chain_update {
            ChainUpdate::ImmutableBlock(data) => {
                let block = data.decode()?;

                info!(
                    "New IMMUTABLE block NUMBER={} SLOT={} HASH={}",
                    block.number(),
                    block.slot(),
                    hex::encode(block.hash()),
                );
            },
            ChainUpdate::Block(data) => {
                let block = data.decode()?;

                info!(
                    "New block NUMBER={} SLOT={} HASH={}",
                    block.number(),
                    block.slot(),
                    hex::encode(block.hash()),
                );
            },
            ChainUpdate::Rollback(data) => {
                let block = data.decode()?;

                info!(
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
