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

    let data_vec = follower
        .read_block_range(
            Point::Specific(
                110_908_236,
                hex::decode("ad3798a1db2b6097c71f35609399e4b2ff834f0f45939803d563bf9d660df2f2")?,
            ),
            Point::Specific(
                110_908_582,
                hex::decode("16e97a73e866280582ee1201a5e1815993978eede956af1869b0733bedc131f2")?,
            ),
        )
        .read()
        .await?;

    let mut total_txs = 0;
    for data in data_vec {
        let block = data.decode()?;
        total_txs += block.tx_count();
    }

    println!("Total transactions: {total_txs}");

    Ok(())
}
