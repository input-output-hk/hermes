//! This example shows how to use the chain follower to follow chain updates on
//! a Cardano network chain.

use std::error::Error;

use cardano_chain_follower::{ChainUpdate, Follower, FollowerConfigBuilder, Network};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Defaults to start following from the tip.
    let config = FollowerConfigBuilder::default().build();

    let mut follower = Follower::connect(
        "relays-new.cardano-mainnet.iohk.io:3001",
        Network::Mainnet,
        config,
    )
    .await?;

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
}
