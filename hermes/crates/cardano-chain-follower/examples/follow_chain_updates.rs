//! This example shows how to use the chain follower to follow chain updates on
//! a Cardano network chain.

use std::error::Error;

use cardano_chain_follower::{ChainUpdate, Client, Follower, Network};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let client =
        Client::connect_n2n("relays-new.cardano-mainnet.iohk.io:3001", Network::Mainnet).await?;
    let mut follower = Follower::new(client);

    loop {
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
            },
            ChainUpdate::Rollback(point) => {
                println!("Rollback");

                // Set the read-pointer to the rollback point.
                follower.set_read_pointer(point).await?;
            },
        }
    }
}
