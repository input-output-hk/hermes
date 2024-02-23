//! This example shows how to use the chain reader to download arbitrary blocks
//! from the chain.

use std::error::Error;

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

    let config = FollowerConfigBuilder::default().build();

    let follower = Follower::connect(
        "relays-new.cardano-mainnet.iohk.io:3001",
        Network::Mainnet,
        config,
    )
    .await?;

    let data = follower
        .read_block(Point::Specific(
            110_908_236,
            hex::decode("ad3798a1db2b6097c71f35609399e4b2ff834f0f45939803d563bf9d660df2f2")?,
        ))
        .read()
        .await?;

    let block = data.decode()?;

    let total_fee = block
        .txs()
        .iter()
        .map(|tx| tx.fee().unwrap_or_default())
        .sum::<u64>();

    println!("Total fee: {total_fee}");

    Ok(())
}
