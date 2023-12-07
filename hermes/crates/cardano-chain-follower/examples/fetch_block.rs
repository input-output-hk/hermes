//! This example shows how to fetch a block from a Cardano network chain
//! using the chain follower.

use std::error::Error;

use cardano_chain_follower::{Client, Follower, Network, Point};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let client =
        Client::connect_n2n("relays-new.cardano-mainnet.iohk.io:3001", Network::Mainnet).await?;
    let mut follower = Follower::new(client);

    let data = follower.client().fetch_block(Point::Origin).await?;

    let block = data.decode()?;
    println!("{}", hex::encode(block.hash()));

    Ok(())
}
