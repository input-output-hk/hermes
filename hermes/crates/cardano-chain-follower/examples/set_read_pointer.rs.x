//! This example shows how to set the follower's read pointer without stopping it.

use std::error::Error;

use cardano_chain_follower::{ChainUpdate, FollowerConfigBuilder, Network, Point};
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

    let (tx, mut rx) = tokio::sync::oneshot::channel::<()>();
    let mut pointer_set = false;
    tokio::spawn(async move {
        let _tx = tx;
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    });

    loop {
        tokio::select! {
            _ = &mut rx, if !pointer_set => {
                follower.set_read_pointer(Point::Specific(
                    110_908_236,
                    hex::decode("ad3798a1db2b6097c71f35609399e4b2ff834f0f45939803d563bf9d660df2f2")?,
                )).await?;
                info!("set read pointer");

                pointer_set = true;
            }

            chain_update = follower.next() => {
                match chain_update? {
                    ChainUpdate::ImmutableBlock(data) | ChainUpdate::ImmutableBlockRollback(data) => {
                        let block = data.decode()?;

                        info!(
                            "New IMMUTABLE block NUMBER={} SLOT={} HASH={}",
                            block.number(),
                            block.slot(),
                            hex::encode(block.hash()),
                        );
                    },
                    ChainUpdate::Block(data) | ChainUpdate::BlockTip(data) => {
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
        }
    }
}
