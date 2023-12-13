//! This example shows how to use the chain follower to download arbitrary blocks
//! from the chain.

use std::error::Error;

use cardano_chain_follower::{ConfigBuilder, Follower, Network, Point};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let config = ConfigBuilder::default().build();

    let mut follower = Follower::connect(
        "relays-new.cardano-mainnet.iohk.io:3001",
        Network::Mainnet,
        config,
    )
    .await?;

    let data = follower
        .fetch_block(Point::Specific(
            110908236,
            hex::decode("ad3798a1db2b6097c71f35609399e4b2ff834f0f45939803d563bf9d660df2f2")?,
        ))
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
