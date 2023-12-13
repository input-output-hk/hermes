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

    let datas = follower
        .fetch_block_range(
            Point::Specific(
                110908236,
                hex::decode("ad3798a1db2b6097c71f35609399e4b2ff834f0f45939803d563bf9d660df2f2")?,
            ),
            Point::Specific(
                110908582,
                hex::decode("16e97a73e866280582ee1201a5e1815993978eede956af1869b0733bedc131f2")?,
            ),
        )
        .await?;

    let mut total_txs = 0;
    for data in datas {
        let block = data.decode()?;
        total_txs = block.tx_count();
    }

    println!("Total transactions: {total_txs}");

    Ok(())
}
