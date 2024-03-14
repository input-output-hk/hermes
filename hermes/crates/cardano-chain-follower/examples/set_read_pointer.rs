//! This example shows how to use the chain follower to follow chain updates on
//! a Cardano network chain.

use std::error::Error;

use cardano_chain_follower::{ChainUpdate, Follower, FollowerConfigBuilder, Network, Point};
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
    let config = FollowerConfigBuilder::default().build();

    let mut follower = Follower::connect(
        "relays-new.cardano-mainnet.iohk.io:3001",
        Network::Mainnet,
        config,
    )
    .await?;

    let mut starting_point = Point::Origin;

    for _ in 0..2 {
        for _ in 0..3 {
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

                    if starting_point == Point::Origin {
                        starting_point = Point::Specific(block.slot(), block.hash().to_vec());
                    }
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

        // Set the read pointer back to the first point we received.
        // This means the next chain update will be the block right after the starting point.
        follower.set_read_pointer(starting_point.clone()).await?;
    }

    // Waits for the follower background task to exit.
    follower.close().await?;

    Ok(())
}
