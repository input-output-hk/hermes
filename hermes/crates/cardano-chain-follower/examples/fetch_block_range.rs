//! This example shows how to fetch a range of blocks from a Cardano network chain
//! using the chain follower.

use std::error::Error;

use cardano_chain_follower::{Client, Follower, Network, Point};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let client =
        Client::connect_n2n("relays-new.cardano-mainnet.iohk.io:3001", Network::Mainnet).await?;
    let mut follower = Follower::new(client);

    let range_data = follower
        .client()
        .fetch_block_range(Point::Origin, Point::Origin)
        .await?;

    for data in range_data {
        let block = data.decode()?;
        println!("{}", hex::encode(block.hash()));
    }

    Ok(())
}
