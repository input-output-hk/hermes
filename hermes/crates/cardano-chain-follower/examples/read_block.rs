//! This example shows how to use the chain follower to download arbitrary blocks
//! from the chain.

use std::error::Error;

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

    let follower = FollowerConfigBuilder::default_for(Network::Mainnet)
        .build()
        .connect()
        .await?;

    let data = follower
        .read_block(Point::Specific(
            110_908_236,
            hex::decode("ad3798a1db2b6097c71f35609399e4b2ff834f0f45939803d563bf9d660df2f2")?,
        ))
        .await?;

    let block = data.decode()?;

    let total_fee = block
        .txs()
        .iter()
        .map(|tx| tx.fee().unwrap_or_default())
        .sum::<u64>();

    info!("Total fee: {total_fee}");

    Ok(())
}
