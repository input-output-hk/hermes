//! This example shows how to use the chain follower to follow chain updates on
//! a Cardano network chain.

use std::error::Error;

use cardano_chain_follower::{ChainUpdate, ConfigBuilder, Follower, Network, PointOrTip};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let config = ConfigBuilder::default().build();

    let mut follower = Follower::connect(
        "relays-new.cardano-mainnet.iohk.io:3001",
        Network::Mainnet,
        config,
    )
    .await?;

    follower.set_read_pointer(PointOrTip::Tip).await?;

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
